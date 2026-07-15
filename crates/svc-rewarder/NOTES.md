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



### BEGIN NOTE - JUNE 25 2026 - 19:05 CST

Yes — this round is green for **`svc-rewarder + svc-storage`**.

The terminal confirms the storage lifetime fix passed, `svc-storage` Phase 4 artifact boundary passed 4/4, `svc-storage` tooling passed 4/4, `svc-rewarder` discovered and passed 16 focused QuickChain tests, and `svc-storage` discovered and passed 19 focused QuickChain tests. Final gate markers show:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=16 ==
== svc-storage quickchain exhaustive preflight gate passed: tests=19 ==
```

The terminal also preserved the forbidden-scope markers: no roots, checkpoints, validators, settlement, anchors, bridges, staking, or liquidity; `svc-rewarder` remains deterministic payout planning only, and `svc-storage` remains bytes by canonical b3 only with cache not acting as paid-access authority. 

# QuickChain Phase 4 Round 1 Crate Notes — `svc-rewarder + svc-storage`

## 0. Status

Crate pair:

```text
svc-rewarder + svc-storage
```

Phase / round:

```text
QuickChain Phase 4 — bonded safety model
Round 1 — bond boundary hardening / no live slashing / no public staking / no liquidity
```

Status:

```text
COMPLETE / PARKED for Phase 4 Round 1
```

This does **not** mean all of Phase 4 is complete. It means the third crate pair in the normal Phase 4 Round 1 sweep is complete and should not be reopened unless a later pair exposes a real boundary regression.

Current Phase 4 Round 1 progress:

```text
ron-proto + ron-ledger              ✅ complete / parked
svc-wallet + ron-accounting         ✅ complete / parked
svc-rewarder + svc-storage          ✅ complete / parked
svc-gateway + omnigate              ⏭ next
svc-index + ron-policy              pending
CrabLink Tauri + client adapters    pending
```

---

## 1. What This Pair Proves

This pass proves that **bond-shaped Phase 4 data can reach planning/storage-adjacent seams without becoming authority**.

`svc-rewarder` remains:

```text
deterministic payout planning only
wallet handoff DTO producer only
not bond truth
not slash truth
not validator reward authority
not public staking authority
not liquidity authority
not wallet mutation truth
not ledger mutation truth
not bridge or external settlement authority
```

`svc-storage` remains:

```text
bytes/artifacts by canonical b3 only
paid admission and metering support only
not bond truth
not slash truth
not wallet truth
not ledger truth
not validator truth
not paid-unlock authority
not cache-only unlock authority
not bridge or external settlement authority
```

The important doctrine is intact:

```text
svc-wallet remains the mutation front-door.
ron-ledger remains economic truth.
ron-accounting remains derivative reporting/snapshot material.
svc-rewarder plans but never mutates ledger truth.
svc-storage stores bytes but never authorizes economic lifecycle decisions.
```

---

## 2. Files Added / Updated

### `svc-rewarder`

Added:

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_planning_boundary.rs
```

Updated:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

The new test suite verifies:

```text
rewarder manifests and wallet handoffs remain planning artifacts
ComputeEpochRequest rejects Phase 4 bond/slash/staking/liquidity authority fields
AccountingSnapshot rejects those fields
RewardPolicy rejects those fields
RewardManifest rejects those fields
SettlementBatch rejects those fields
WalletIssueRequest rejects those fields
rewarder replay/dedupe does not become second payout or slash authority
rewarder source does not implement bond, slash, staking, liquidity, bridge, external settlement, or ron-ledger authority
```

Important fix applied during this round:

```text
The rewarder test fixture originally used broad substring checks against "bond".
That falsely failed because the test data itself included "bond" in epoch/account/salt names.
The fixture was renamed to neutral "reward-planning" vocabulary, and exact authority-key checks were kept.
```

Final focused result:

```text
quickchain_phase4_bond_planning_boundary: 5 passed
quickchain_tooling_boundary: 4 passed
full svc-rewarder preflight: tests=16 passed
all-target tests: passed
clippy: passed
```

---

## 3. `svc-rewarder` Boundary Now Locked

The crate now explicitly rejects or scans against Phase 4 authority creep such as:

```text
QuickChainValidatorBondAccount
QuickChainSlashEvidence
bond_lifecycle_decision
bond_account_status
apply_bond
commit_bond
execute_bond
apply_slash
commit_slash
execute_slash
slash_bond
auto_slash_now
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
solana
rox
ron_ledger::
```

Allowed posture:

```text
rewarder may consume accounting/policy inputs for deterministic payout planning
rewarder may produce wallet issue handoff DTOs
rewarder may dedupe replay by run key
rewarder may preview or emit through svc-wallet routes where already designed
```

Forbidden posture remains:

```text
rewarder must not mutate wallet directly
rewarder must not mutate ledger directly
rewarder must not produce roots/checkpoints/finality
rewarder must not slash bonds
rewarder must not create validator rewards
rewarder must not create public staking markets
rewarder must not touch liquidity, bridge, ROX, Solana, or external settlement
```

---

## 4. `svc-storage`

Added:

```text
crates/svc-storage/tests/quickchain_phase4_bond_artifact_boundary.rs
```

Updated:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

The new test suite verifies:

```text
AccountingExportRequest rejects Phase 4 bond/slash/staking/liquidity authority fields
UsageEventDto rejects nested authority fields
usage events remain metering only
storage can retain opaque bond/evidence/report-like artifact bytes by canonical b3
b3 proves bytes only, not lifecycle authority
storage source does not implement Phase 4 bond/slash/staking/liquidity/bridge/external settlement runtime authority
```

Important fix applied during this round:

```text
AccountingExportRequest and UsageEventDto contain &'static str fields.
The generated rejection JSON strings had to be converted into leaked &'static str test fixtures with Box::leak(...into_boxed_str()).
That fixed the E0597 lifetime failure without changing production code.
```

Final focused result:

```text
quickchain_phase4_bond_artifact_boundary: 4 passed
quickchain_tooling_boundary: 4 passed
full svc-storage preflight: tests=19 passed
all-target tests: passed
clippy: passed
```

---

## 5. `svc-storage` Boundary Now Locked

The crate now explicitly rejects or scans against Phase 4 authority creep such as:

```text
QuickChainValidatorBondAccount
QuickChainSlashEvidence
bond_lifecycle_decision
bond_account_status
apply_bond
commit_bond
execute_bond
apply_slash
commit_slash
execute_slash
slash_bond
auto_slash_now
validator_reward_receipt
public_staking_market
liquidity_pool
paid_unlock_from_bond
unlock_from_bond_artifact
cache_only_unlock
bridge_settlement
external_settlement
solana
rox
ron_ledger::
```

Allowed posture:

```text
storage may store opaque bytes
storage may serve canonical b3 content
storage may support paid admission paths
storage may emit metering/accounting export DTOs
storage may hold future proof/evidence/report artifacts as bytes
```

Forbidden posture remains:

```text
storage must not treat b3 as payment proof
storage must not treat cache as entitlement
storage must not unlock paid content from artifact bytes alone
storage must not generate wallet receipts
storage must not become bond truth or slash truth
storage must not mutate wallet or ledger truth
storage must not produce roots/finality/checkpoints
storage must not introduce public staking, liquidity, bridge, Solana, ROX, or external settlement
```

---

## 6. Commands That Passed

Focused repair / validation:

```text
cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary
cargo test -p svc-rewarder --test quickchain_tooling_boundary
cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary
cargo test -p svc-storage --test quickchain_tooling_boundary
```

Full crate-local preflights:

```text
bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
bash crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final gate markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=16 ==
== svc-storage quickchain exhaustive preflight gate passed: tests=19 ==
```

---

## 7. Parked Status

Final status for this pair:

```text
QuickChain Phase 4 Round 1 — svc-rewarder + svc-storage
Status: COMPLETE / PARKED
```

Do not reopen these crates for Phase 4 Round 1 unless:

```text
a later crate-pair exposes a real integration mismatch
a scanner catches new authority vocabulary drift
svc-wallet / ron-ledger bond DTOs change
gateway / omnigate paid enforcement contracts require stricter storage/rewarder boundary checks
the buildplan explicitly opens a later Phase 4 round for this pair
```

---

## 8. Next Crate Pair

Next normal crate pair:

```text
svc-gateway + omnigate
```

Expected Phase 4 Round 1 mission for the next pair:

```text
svc-gateway remains public/API boundary only.
omnigate remains hydration/enforcement coordinator only.
Neither becomes bond truth, slash truth, wallet truth, ledger truth, public staking authority, liquidity authority, bridge authority, or external settlement authority.
Paid access must still follow backend wallet/ledger receipt paths.
No cache-only unlock.
No silent spend.
No fake receipts.
No ROX/Solana/public bridge creep.
```


### END NOTE - JUNE 25 2026 - 19:05 CST


### BEGIN NOTE - JUNE 26 2026 - 01:35 CST

Here are the crate notes for the completed `svc-rewarder + svc-storage` slice. Phase 4 Round 2 is specifically the slashing/challenge **simulation** round, with challenge windows, appeal/freeze states, replayable dispute state, and **no live irreversible slash yet**. 

# QuickChain Phase 4 Round 2 Crate Notes — `svc-rewarder` + `svc-storage`

Date: June 26, 2026
Project: RustyOnions / CrabLink
Phase / round: QuickChain Phase 4 Round 2
Crate pair: `svc-rewarder + svc-storage`
Status: COMPLETE / PARKED / GREEN

## 0. Final status

`svc-rewarder + svc-storage` are complete and parked for QuickChain Phase 4 Round 2.

Final status:

```text
QuickChain Phase 4 Round 2 — svc-rewarder + svc-storage
Status: COMPLETE / PARKED / GREEN
```

Final proof markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=17 ==
== svc-rewarder QuickChain parking gate passed ==

== svc-storage quickchain exhaustive preflight gate passed: tests=20 ==
== svc-storage QuickChain parking gate passed ==
```

The final `svc-rewarder` run shows the clippy fix passed, all 17 focused QuickChain tests ran, all-target tests passed, clippy passed, and the final parking gate passed.  The prior run shows `svc-storage` discovered 20 focused QuickChain tests, passed all-target tests, passed clippy, and reached its final parking gate. 

Current Phase 4 Round 2 crate-pair position:

```text
1. ron-proto + ron-ledger              COMPLETE / PARKED
2. svc-wallet + ron-accounting         COMPLETE / PARKED
3. svc-rewarder + svc-storage          COMPLETE / PARKED
4. svc-gateway + omnigate              NEXT
5. svc-index + ron-policy              pending
6. CrabLink Tauri + client adapters    pending
```

## 1. Phase 4 Round 2 context

Phase 4 Round 2 is the **slashing/challenge simulation** round.

Allowed scope:

```text
challenge windows
slash evidence validation
appeal/freeze states
hold-like lifecycle for disputed bonds
replayable dispute state
safe dispute simulation
no one-step irreversible slash
```

Exit gate:

```text
slashing simulation is deterministic
slashing simulation is replayable
challenge path is safe
dispute windows are explicit
no live irreversible slash yet
```

For this crate pair, the correct downstream shape was:

```text
svc-rewarder:
  reject dispute/slash/challenge simulation as reward or payout authority

svc-storage:
  store dispute/evidence bytes only as opaque b3-addressed artifacts
```

This pair did not implement live slashing, bond enforcement, validator rewards, staking, liquidity, bridges, ROX/Solana, external settlement, payout execution, or storage unlock authority.

## 2. Doctrine preserved

The following boundaries remain intact:

```text
svc-rewarder plans payouts but never mutates ledger
svc-storage stores bytes/artifacts but is not payment truth
svc-wallet remains mutation front-door
ron-ledger remains economic truth
ron-accounting remains derivative reporting/snapshots, not balance truth
storage cache is convenience, not paid-access authority
b3 proves bytes, not economic truth
dispute evidence artifacts do not become slash truth
reward plans do not become receipts
wallet handoff DTOs do not become rewarder-created receipts
```

Forbidden scope still absent:

```text
no live irreversible slash
no automatic slash
no slash reward
no penalty reward
no validator reward receipt
no bond dispute payout
no wallet mutation from rewarder
no ledger mutation from rewarder
no paid unlock from storage evidence
no cache-only unlock
no storage-created entitlement truth
no staking market
no liquidity pool
no bridge settlement
no external settlement
no ROX/Solana active runtime
```

## 3. `svc-rewarder` changes

### Files added / updated

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_dispute_reward_boundary.rs
crates/svc-rewarder/docs/quickchain-preflight.md
```

Script executable bits were also ensured for:

```text
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The uploaded `svc-rewarder` bundle confirms the crate already had QuickChain-focused tests and dynamic park/preflight scripts before this patch. 

### New test file

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_dispute_reward_boundary.rs
```

Purpose:

```text
Prove disputed-bond simulation cannot become rewarder payout, slash, penalty, wallet, ledger, staking, liquidity, bridge, or external settlement authority.
```

The new test suite contains 4 tests:

```text
dispute_simulation_fields_reject_at_rewarder_input_boundaries
dispute_simulation_fields_reject_at_rewarder_output_and_wallet_handoff_boundaries
rewarder_does_not_emit_dispute_reward_when_scores_are_zero_or_dry_run
rewarder_source_does_not_construct_phase4_round2_dispute_runtime_authority
```

Focused result:

```text
cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary

Result:
  4 passed
  0 failed
```

The terminal output confirms all four tests passed. 

### Boundary keys rejected by `svc-rewarder`

The new tests reject Phase 4 Round 2 dispute/slash authority fields at these rewarder boundaries:

```text
ComputeEpochRequest
AccountingSnapshot
RewardPolicy
RewardManifest
SettlementBatch
WalletIssueRequest
```

Rejected authority field families include:

```text
dispute_id
dispute_status
challenge_window
challenge_window_open
appeal_window
appeal_window_open
freeze_pending_appeal
frozen_minor
disputed_minor
slash_evidence
slash_decision
slash_recommendation
slash_reward
slash_reward_recipient
slash_penalty_minor
automatic_slash
auto_slash_now
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
bond_penalty
bond_dispute_payout
dispute_reward
reward_from_dispute
penalty_reward
validator_reward
validator_reward_receipt
wallet_receipt
ledger_receipt
wallet_mutation
ledger_mutation
payout_execution
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
solana
rox
```

Important meaning:

```text
Dispute simulation cannot enter rewarder as a payout basis.
Dispute simulation cannot enter rewarder outputs as a reward plan.
Dispute simulation cannot enter wallet handoff DTOs as wallet authority.
```

### Source scanner added for `svc-rewarder`

The new source scanner verifies `svc-rewarder/src` does not construct runtime authority through strings like:

```text
reward_from_dispute
dispute_reward
slash_reward
penalty_reward
bond_dispute_payout
payout_from_dispute
payout_from_slash
slash_penalty_minor
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
wallet_slash
ledger_slash
auto_slash_now
freeze_bond_payout
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
```

This preserves:

```text
svc-rewarder = deterministic payout planning only
not dispute authority
not slash authority
not wallet authority
not ledger authority
not validator reward authority
not public staking authority
not bridge/external settlement authority
```

## 4. `svc-rewarder` docs updated

The `svc-rewarder` QuickChain runbook was appended with a Phase 4 Round 2 section.

New documented boundary:

```text
challenge/freeze/appeal/slash simulation must not create reward payouts
disputed-bond state must not become validator reward authority
slash evidence must not become payout authority
rejected slash simulation must not become protocol reward input
rewarder must not mutate wallet or ledger state
rewarder must not produce bond/slash/penalty receipts
rewarder must not create staking, liquidity, bridge, ROX, Solana, or external settlement behavior
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

This keeps docs aligned with the new test boundary.

## 5. `svc-rewarder` issue encountered and fixed

### Clippy failure

After focused tests passed, `svc-rewarder` park stopped during clippy with three lints:

```text
unnecessary use of to_owned
unnecessary use of to_owned
assert_eq! with literal bool
```

The failing lines were in:

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_dispute_reward_boundary.rs
```

Fix applied:

```text
ContentCid::parse(INPUTS_CID.to_owned())
->
ContentCid::parse(INPUTS_CID)

assert_eq!(manifest.ledger.emitted, false)
->
assert!(!manifest.ledger.emitted)
```

The fix passed:

```text
cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

The terminal confirms the focused test passed and clippy finished cleanly. 

## 6. `svc-rewarder` final gate

Final `svc-rewarder` preflight discovered 17 QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_phase3_validator_lifecycle_boundary
quickchain_phase4_bond_dispute_reward_boundary
quickchain_phase4_bond_planning_boundary
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

Final marker:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=17 ==
== svc-rewarder QuickChain parking gate passed ==
```

The final output confirms this exact result. 

Important count change:

```text
Phase 4 Round 1 svc-rewarder count: 16
Phase 4 Round 2 svc-rewarder count: 17
```

The new test target is:

```text
quickchain_phase4_bond_dispute_reward_boundary
```

## 7. What `svc-rewarder` now proves

`svc-rewarder` now proves:

```text
dispute simulation cannot become rewarder payout basis
challenge/freeze/appeal/slash fields reject at request input boundary
challenge/freeze/appeal/slash fields reject at accounting snapshot boundary
challenge/freeze/appeal/slash fields reject at reward policy boundary
challenge/freeze/appeal/slash fields reject at reward manifest boundary
challenge/freeze/appeal/slash fields reject at settlement batch boundary
challenge/freeze/appeal/slash fields reject at wallet issue handoff boundary
zero-score and dry-run dispute-adjacent planning emits no dispute reward
rewarder source does not construct slash/reward/penalty/bond-dispute payout authority
rewarder remains deterministic payout planning only
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

## 8. What `svc-rewarder` still does not implement

`svc-rewarder` does not implement:

```text
slash rewards
penalty rewards
validator rewards from dispute state
bond dispute payouts
live slashing
automatic slashing
wallet freezing
wallet mutation
ledger mutation
bond capture
bond forfeiture
payout execution from dispute state
public staking market
liquidity pool
bridge settlement
external settlement
ROX/Solana runtime
```

## 9. `svc-storage` changes

### Files added / updated

```text
crates/svc-storage/tests/quickchain_phase4_dispute_evidence_artifact_boundary.rs
crates/svc-storage/docs/quickchain-preflight.md
```

Script executable bits were also ensured for:

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The uploaded `svc-storage` bundle confirms the crate already had dynamic QuickChain park/preflight scripts and prior Phase 4 bond artifact tests. 

### New test file

```text
crates/svc-storage/tests/quickchain_phase4_dispute_evidence_artifact_boundary.rs
```

Purpose:

```text
Prove dispute/challenge/evidence bytes are opaque b3-addressed artifacts only, never unlock, slash, wallet, ledger, bond, staking, liquidity, bridge, or external settlement authority.
```

The new test suite contains 4 tests:

```text
storage_accounting_export_rejects_dispute_authority_fields
storage_usage_event_remains_metering_not_dispute_or_unlock_authority
dispute_evidence_bytes_store_by_b3_without_unlock_or_slash_authority
storage_source_does_not_construct_phase4_round2_dispute_runtime_or_unlock_authority
```

Focused result:

```text
cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary

Result:
  4 passed
  0 failed
```

The terminal output confirms all four tests passed. 

### Boundary keys rejected by `svc-storage`

The new tests reject Phase 4 Round 2 dispute/unlock/slash authority fields at storage metering/export boundaries:

```text
AccountingExportRequest
UsageEventDto
nested UsageEventDto inside AccountingExportRequest
```

Rejected authority field families include:

```text
dispute_id
dispute_status
challenge_window
challenge_window_open
appeal_window
appeal_window_open
freeze_pending_appeal
frozen_minor
disputed_minor
slash_evidence
slash_decision
slash_recommendation
slash_capture
automatic_slash
auto_slash_now
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
bond_penalty
wallet_receipt
ledger_receipt
wallet_mutation
ledger_mutation
paid_unlock
paid_unlock_from_dispute
unlock_from_evidence
cache_only_unlock
cache_unlock_authority
validator_reward
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
solana
rox
```

Important meaning:

```text
Storage usage events stay metering-only.
Accounting exports stay metering-only.
Evidence metadata cannot smuggle paid unlock or slash authority.
```

### Opaque b3 artifact proof

The new storage test stores this kind of body as bytes:

```json
{"schema":"quickchain.dispute-evidence.bytes-only.test","status":"opaque"}
```

It verifies:

```text
bytes are stored under canonical b3
full read returns same bytes
range read returns bounded bytes
head returns length and content-derived etag
etag remains content-hash-derived
no unlock/slash authority is inferred
```

Meaning:

```text
b3 proves the bytes stored.
b3 does not prove slash truth.
b3 does not prove bond truth.
b3 does not unlock paid content.
```

### Source scanner added for `svc-storage`

The new source scanner verifies `svc-storage/src` does not construct runtime authority through strings like:

```text
dispute_unlock
paid_unlock_from_dispute
unlock_from_dispute
unlock_from_evidence
evidence_paid_unlock
cache_dispute_authority
cache_unlock_authority_from_dispute
slash_evidence_truth
bond_dispute_truth
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
wallet_slash
ledger_slash
auto_slash_now
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
```

This preserves:

```text
svc-storage = bytes/artifacts by b3 only
not dispute authority
not slash authority
not bond truth
not paid unlock authority
not cache entitlement authority
not wallet authority
not ledger authority
not public staking authority
not bridge/external settlement authority
```

## 10. `svc-storage` docs updated

The `svc-storage` QuickChain runbook was appended with a Phase 4 Round 2 section.

New documented boundary:

```text
b3 proves bytes only
stored evidence bytes do not become slash truth
stored evidence bytes do not become bond truth
stored evidence bytes do not unlock paid content
cache does not become entitlement authority
accounting export remains metering only
storage must not mutate wallet or ledger state
storage must not create staking, liquidity, bridge, ROX, Solana, or external settlement behavior
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

This keeps docs aligned with the new test boundary.

## 11. `svc-storage` final gate

Final `svc-storage` preflight discovered 20 QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_phase3_validator_lifecycle_boundary
quickchain_phase4_bond_artifact_boundary
quickchain_phase4_dispute_evidence_artifact_boundary
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

Final marker:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=20 ==
== svc-storage QuickChain parking gate passed ==
```

The terminal output confirms this exact result. 

Important count change:

```text
Phase 4 Round 1 svc-storage count: 19
Phase 4 Round 2 svc-storage count: 20
```

The new test target is:

```text
quickchain_phase4_dispute_evidence_artifact_boundary
```

## 12. What `svc-storage` now proves

`svc-storage` now proves:

```text
dispute evidence artifacts are opaque bytes only
dispute evidence artifacts store and read by canonical b3
b3 does not become slash truth
b3 does not become bond truth
b3 does not become unlock authority
cache does not become entitlement authority
accounting export rejects dispute/slash/unlock authority fields
usage event DTO rejects dispute/slash/unlock authority fields
storage source does not construct dispute runtime authority
storage source does not construct slash authority
storage source does not construct paid unlock authority from evidence
storage source does not construct staking, liquidity, bridge, or external settlement authority
```

## 13. What `svc-storage` still does not implement

`svc-storage` does not implement:

```text
slash evidence truth
bond dispute truth
paid unlock from evidence
cache-only unlock
wallet mutation
ledger mutation
automatic slashing
bond capture
bond forfeiture
validator rewards
staking market
liquidity pool
bridge settlement
external settlement
ROX/Solana runtime
```

## 14. Focused tests run

Focused tests that passed for this pair:

```bash
cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary
cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary
cargo test -p svc-rewarder --test quickchain_tooling_boundary

cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary
cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary
cargo test -p svc-storage --test quickchain_tooling_boundary
```

Focused results:

```text
svc-rewarder quickchain_phase4_bond_dispute_reward_boundary:
  4 passed
  0 failed

svc-rewarder quickchain_phase4_bond_planning_boundary:
  5 passed
  0 failed

svc-rewarder quickchain_tooling_boundary:
  4 passed
  0 failed

svc-storage quickchain_phase4_dispute_evidence_artifact_boundary:
  4 passed
  0 failed

svc-storage quickchain_phase4_bond_artifact_boundary:
  4 passed
  0 failed

svc-storage quickchain_tooling_boundary:
  4 passed
  0 failed
```

The terminal output confirms the focused tests passed before park gates were run. 

## 15. Final park commands used

```bash
cd /Users/mymac/Desktop/RustyOnions

crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

Final result:

```text
svc-rewarder: COMPLETE / PARKED / GREEN
svc-storage: COMPLETE / PARKED / GREEN
```

## 16. Safe status wording

Use this wording:

```text
svc-rewarder + svc-storage are COMPLETE / PARKED for QuickChain Phase 4 Round 2.

svc-rewarder now rejects disputed-bond challenge/freeze/appeal/slash simulation as reward, payout, wallet, ledger, staking, liquidity, bridge, or external settlement authority.

svc-storage now treats dispute/evidence artifacts as opaque b3-addressed bytes only and rejects any attempt to turn those bytes, cache state, accounting export, or usage events into slash truth, bond truth, paid unlock authority, wallet authority, ledger authority, staking, liquidity, bridge, or external settlement authority.
```

Do not say:

```text
slashing is live
bond enforcement is live
validator rewards are live
rewarder pays slashing rewards
storage verifies slash evidence truth
storage unlocks paid content from evidence
cache can unlock paid content
staking is live
liquidity is live
bridge is ready
external settlement is active
ROX/Solana is active
```

## 17. Retest commands if reopened

Only rerun if these crates are modified again.

Focused retests:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary
cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary
cargo test -p svc-rewarder --test quickchain_tooling_boundary
cargo clippy -p svc-rewarder --all-targets -- -D warnings

cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary
cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary
cargo test -p svc-storage --test quickchain_tooling_boundary
cargo clippy -p svc-storage --all-targets -- -D warnings
```

Full park gates:

```bash
cd /Users/mymac/Desktop/RustyOnions

crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

Expected final markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=17 ==
== svc-rewarder QuickChain parking gate passed ==

== svc-storage quickchain exhaustive preflight gate passed: tests=20 ==
== svc-storage QuickChain parking gate passed ==
```

## 18. Low-disk note

This pair is parked.

Do not rerun these gates unless:

```text
svc-rewarder is modified
svc-storage is modified
a downstream pair exposes a compile failure
a downstream pair exposes a real boundary regression
a final Phase 4 Round 2 audit needs targeted confirmation
```

Otherwise, move forward.

## 19. Next crate pair

Next active pair:

```text
svc-gateway + omnigate
```

Recommended codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Expected focus for `svc-gateway + omnigate` in Phase 4 Round 2:

```text
gateway must not expose slash/dispute execution routes
gateway must not turn dispute reports into paid unlock authority
gateway must not mutate wallet or ledger state
gateway must not claim settlement/finality/slash truth
gateway must keep paid enforcement backend-derived

omnigate must hydrate/display dispute state only if needed
omnigate must not become dispute truth
omnigate must not execute slash/capture/freeze/reward behavior
omnigate must not turn cache or hydration into paid unlock authority
omnigate must not create bridge, staking, liquidity, ROX/Solana, or external settlement behavior
```

## 20. Completion judgment

Final judgment:

```text
svc-rewarder + svc-storage are complete / parked for QuickChain Phase 4 Round 2.
```

This pair now has:

```text
rewarder disputed-bond reward boundary tests
storage dispute evidence artifact boundary tests
docs updated for Phase 4 Round 2 dispute boundaries
strict unknown-field rejection at rewarder input/output/wallet handoff surfaces
strict unknown-field rejection at storage accounting export/usage event surfaces
source scanners for dispute/slash/reward/unlock/staking/liquidity/bridge/external settlement authority
b3-only evidence artifact proof
clippy fix for rewarder test
full svc-rewarder park green
full svc-storage park green
no live slashing
no dispute payout
no validator reward
no storage unlock authority
no cache entitlement authority
no wallet/ledger mutation from these crates
no staking
no liquidity
no bridge
no external settlement
no ROX/Solana
```

Next active pair:

```text
svc-gateway + omnigate
```

### END NOTE — JUNE 26 2026 — QUICKCHAIN PHASE 4 ROUND 2 — `svc-rewarder + svc-storage`


### END NOTE - JUNE 26 2026 - 01:35 CST




### BEGIN NOTE - JUNE 26 2026 - 16:25 CST

QuickChain Phase 4 crate notes — svc-rewarder + svc-storage

Status

svc-rewarder + svc-storage are complete for QuickChain Phase 4.

This pair has now cleared all three Phase 4 rounds:

Phase 4 Round 1:
svc-rewarder added bond-planning boundary coverage.
svc-storage added bond-artifact storage boundary coverage.

Phase 4 Round 2:
svc-rewarder added disputed-bond reward boundary coverage.
svc-storage added dispute-evidence artifact boundary coverage.

Phase 4 Round 3:
svc-rewarder added controlled bond-enforcement reward boundary coverage.
svc-storage added controlled bond-enforcement artifact boundary coverage.

Final status:
Both crates are green and parked for Phase 4.

The terminal confirms the final Round 3 marker:

QuickChain Phase 4 Round 3 svc-rewarder + svc-storage clippy fix passed. 

Files touched / important areas

svc-rewarder

crates/svc-rewarder/tests/quickchain_phase4_bond_enforcement_reward_boundary.rs

Added the Phase 4 Round 3 controlled bond-enforcement reward boundary suite.

This test suite proves that controlled bond enforcement cannot become rewarder payout, penalty, wallet, ledger, staking, liquidity, bridge, validator-reward, external-chain, or public-market authority.

The tests cover:

ComputeEpochRequest rejects top-level controlled bond-enforcement authority fields.

Nested RewardPolicy rejects controlled bond-enforcement authority fields.

Nested AccountingSnapshot rejects controlled bond-enforcement authority fields.

RewardManifest remains a deterministic reward planning artifact, not enforcement truth.

SettlementBatch remains a wallet-handoff planning artifact.

WalletIssueBatch remains `/v1/issue` request preview/handoff shape, not bond enforcement, slash, capture, release, or receipt authority.

Rewarder replay/dedupe still prevents duplicate payout and does not become slash/punishment logic.

Rewarder source does not implement controlled bond-enforcement runtime authority.

The final clippy cleanup changed:

ContentCid::parse(INPUTS_CID.to_string())

to:

ContentCid::parse(INPUTS_CID)

This fixed the `unnecessary use of to_string` clippy warning without changing behavior.

crates/svc-rewarder/scripts/dev-quickchain-preflight.sh

No structural script rewrite was needed.

The script dynamically discovered the new Round 3 test automatically.

Final discovered QuickChain tests for svc-rewarder:

18 focused QuickChain tests.

svc-rewarder gates passed

Focused Round 3 test:

cargo test -p svc-rewarder --test quickchain_phase4_bond_enforcement_reward_boundary

Result:

4 passed / 0 failed

Tests inside the focused Round 3 suite:

bond_enforcement_like_replay_is_still_rewarder_dedupe_not_second_payout_or_slash — passed

rewarder_manifest_and_wallet_issue_batch_remain_planning_not_enforcement_truth — passed

rewarder_compute_request_rejects_round3_enforcement_authority_fields — passed

rewarder_source_does_not_implement_phase4_round3_bond_enforcement_authority — passed

Full preflight:

bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh

Result:

svc-rewarder quickchain exhaustive preflight gate passed: tests=18

Additional svc-rewarder gates inside preflight:

forbidden helper scan passed

format check passed

all discovered QuickChain focused tests passed

all-target tests passed

unit tests passed

integration tests passed

bench smoke passed

clippy passed

Important svc-rewarder doctrine preserved

svc-rewarder remains deterministic payout planning only.

svc-rewarder does not mutate ron-ledger.

svc-rewarder does not directly create wallet receipts.

svc-rewarder does not execute controlled bond enforcement.

svc-rewarder does not reserve slash amounts.

svc-rewarder does not release slash reserves.

svc-rewarder does not capture slash reserves.

svc-rewarder does not create validator rewards from enforcement events.

svc-rewarder does not convert bond enforcement into reward eligibility.

svc-rewarder does not become staking, liquidity, bridge, public-market, external-settlement, Solana, or ROX authority.

All economic mutation still goes through svc-wallet, with ron-ledger as truth.

svc-storage

crates/svc-storage/tests/quickchain_phase4_bond_enforcement_artifact_boundary.rs

Added the Phase 4 Round 3 controlled bond-enforcement artifact boundary suite.

This test suite proves that controlled bond-enforcement artifacts may be stored as opaque bytes by canonical b3, but svc-storage does not interpret those bytes as wallet, ledger, paid unlock, slash, capture, release, staking, liquidity, bridge, external settlement, or public-market authority.

The tests cover:

AccountingExportRequest rejects top-level controlled bond-enforcement authority fields.

UsageEventDto rejects nested controlled bond-enforcement authority fields.

Storage usage events remain metering only.

Accounting export remains usage/metering only.

Controlled bond-enforcement artifact bytes can be stored and retrieved by canonical b3.

Stored artifact bytes do not unlock paid content.

Stored artifact bytes do not authorize wallet mutation.

Stored artifact bytes do not authorize ledger mutation.

Stored artifact bytes do not authorize slash, reserve, release, or capture.

Storage source does not implement controlled bond-enforcement runtime authority.

crates/svc-storage/scripts/dev-quickchain-preflight.sh

No structural script rewrite was needed.

The script dynamically discovered the new Round 3 test automatically.

Final discovered QuickChain tests for svc-storage:

21 focused QuickChain tests.

svc-storage gates passed

Focused Round 3 test:

cargo test -p svc-storage --test quickchain_phase4_bond_enforcement_artifact_boundary

Result:

4 passed / 0 failed

Tests inside the focused Round 3 suite:

storage_usage_events_remain_metering_not_bond_enforcement_or_paid_unlock_authority — passed

enforcement_artifact_bytes_store_by_b3_without_unlock_slash_or_wallet_authority — passed

storage_accounting_export_rejects_round3_enforcement_authority_fields — passed

storage_source_does_not_implement_phase4_round3_bond_enforcement_authority — passed

Full preflight:

bash crates/svc-storage/scripts/dev-quickchain-preflight.sh

Result:

svc-storage quickchain exhaustive preflight gate passed: tests=21

Additional svc-storage gates inside preflight:

forbidden helper scan passed

format check passed

all discovered QuickChain focused tests passed

all-target tests passed

paid write tests passed

wallet receipt mode tests passed

settlement boundary tests passed

b3 integrity tests passed

range/media boundary tests passed

bench compile/smoke passed

clippy passed

Important svc-storage doctrine preserved

svc-storage remains bytes/artifacts by canonical b3 only.

b3 proves bytes, not authority.

svc-storage cache is not paid-access authority.

svc-storage cannot unlock paid content from cache alone.

svc-storage does not become wallet truth.

svc-storage does not become ledger truth.

svc-storage does not create receipts.

svc-storage does not execute bond enforcement.

svc-storage does not reserve slash amounts.

svc-storage does not release slash reserves.

svc-storage does not capture slash reserves.

svc-storage does not mutate balances.

svc-storage does not become validator, staking, liquidity, bridge, public-market, external-settlement, Solana, or ROX authority.

Paid storage still requires backend-derived wallet/ledger evidence.

Issues fixed during this pair

1. svc-rewarder clippy warning

Initial Round 3 focused tests passed, but svc-rewarder preflight stopped during clippy on an unnecessary `to_string()` call.

Fix:

Changed:

ContentCid::parse(INPUTS_CID.to_string())

to:

ContentCid::parse(INPUTS_CID)

Result:

svc-rewarder clippy passed.

2. No design or boundary failure

The failure was only a clippy cleanup in the new test file.

No production source authority issue was found.

No rewarder/storage doctrine issue was found.

No forbidden runtime behavior was introduced.

Final confirmed terminal summary

svc-rewarder:

Focused Round 3 test: 4/4 passed

Exhaustive QuickChain preflight: passed

Discovered QuickChain tests: 18

All-target tests: passed

Clippy: passed

svc-storage:

Focused Round 3 test: 4/4 passed

Exhaustive QuickChain preflight: passed

Discovered QuickChain tests: 21

All-target tests: passed

Clippy: passed

Final proof marker:

QuickChain Phase 4 Round 3 svc-rewarder + svc-storage clippy fix passed. 

Conclusion

svc-rewarder + svc-storage are complete for QuickChain Phase 4.

This crate pair is parked.

Next Phase 4 Round 3 crate pair:

svc-gateway + omnigate


### END NOTE - JUNE 26 2026 - 16:25 CST


### BEGIN NOTE - JUNE 27 2026 - 19:05 CST

## Crate Notes — QuickChain Phase 5 Round 1

## Crates: `svc-rewarder` + `svc-storage`

### Session status

```text
QuickChain Phase 5 Round 1 crate pair:
svc-rewarder + svc-storage = COMPLETE / PARKED
```

Both crates are green after the Phase 5 anchor-only evidence/artifact patch and the storage scanner fix. The final terminal output confirms:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=19
svc-rewarder QuickChain parking gate passed

svc-storage quickchain exhaustive preflight gate passed: tests=22
svc-storage QuickChain parking gate passed

QuickChain Phase 5 Round 1 svc-rewarder + svc-storage scanner fix passed.
```



---

# svc-rewarder notes

## Phase 5 Round 1 purpose

`svc-rewarder` now has a Phase 5 Round 1 boundary proving that reward manifest commitments may be referenced as **anchor evidence only**.

This does **not** make rewarder an anchor authority.

This does **not** make rewarder payout authority.

This does **not** make rewarder wallet, ledger, balance, finality, settlement, or outside-chain truth.

The safe interpretation is:

```text
A deterministic reward manifest commitment may be referenced by anchor dry-run evidence.
The report is evidence-only metadata.
svc-rewarder remains deterministic payout planning only.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
```

## Files changed

```text
crates/svc-rewarder/docs/quickchain-preflight.md
crates/svc-rewarder/tests/quickchain_phase5_anchor_evidence_boundary.rs
```

## Added test coverage

```text
quickchain_phase5_anchor_evidence_boundary
```

Focused tests added/proven:

```text
reward_manifest_commitment_can_be_referenced_as_anchor_evidence_only
rewarder_anchor_evidence_rejects_authority_flags_unknown_fields_and_bad_b3
rewarder_phase5_docs_record_anchor_only_non_authority_boundary
rewarder_runtime_source_does_not_gain_phase5_anchor_authority
```

The focused Phase 5 rewarder test passed:

```text
running 4 tests
4 passed
```



## Boundary enforced

The test-local evidence report proves these flags stay safe:

```text
report_only = true
evidence_only = true

rewarder_side_effect = false
wallet_side_effect = false
ledger_side_effect = false
payout_side_effect = false
reward_truth = false
paid_unlock_authority = false
settlement_truth = false
outside_chain_truth = false
```

## Authority fields rejected

The test rejects unknown/poison authority fields such as:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payout_executed
reward_executed
paid_unlock
settlement_status
finalized
bridge_settlement
solana
rox
```

## Runtime source scanner

The new scanner verifies `svc-rewarder/src` does not gain Phase 5 runtime authority patterns such as:

```text
anchor_payout
payout_from_anchor
settle_from_anchor
commit_from_anchor
apply_anchor
anchor_wallet_receipt
anchor_ledger_receipt
paid_unlock_from_anchor
bridge_settlement
external_settlement
solana
rox
liquidity_pool
```

## What svc-rewarder still cannot do

```text
No anchor payout execution.
No anchor-created reward truth.
No wallet mutation.
No ledger mutation.
No direct payout execution.
No fake receipts.
No fake balances.
No paid unlock from anchor evidence.
No settlement truth.
No outside-chain ROC truth.
No bridge.
No ROX runtime.
No Solana runtime.
No staking.
No liquidity.
```

## Green gate summary

```text
Focused Phase 5 test: 4/4 passed
Focused QuickChain tests discovered: 19
All focused QuickChain tests passed
All-targets test passed
Clippy passed
Parking gate passed
```

---

# svc-storage notes

## Phase 5 Round 1 purpose

`svc-storage` now has a Phase 5 Round 1 boundary proving that opaque anchor dry-run artifact bytes may be stored and retrieved by canonical `b3` only.

This does **not** make storage an anchor authority.

This does **not** make storage payment truth.

This does **not** make storage paid-unlock authority.

This does **not** make cache, CIDs, artifacts, or b3 byte identity ROC balance/finality truth.

The safe interpretation is:

```text
svc-storage may store opaque anchor dry-run artifact bytes by canonical b3.
The b3 proves byte identity only.
svc-storage remains bytes/artifact infrastructure.
Paid access remains backend wallet/gateway/omnigate derived.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
```

## Files changed

```text
crates/svc-storage/docs/quickchain-preflight.md
crates/svc-storage/tests/quickchain_phase5_anchor_artifact_boundary.rs
```

## Added test coverage

```text
quickchain_phase5_anchor_artifact_boundary
```

Focused tests added/proven:

```text
storage_can_retain_anchor_dry_run_artifact_bytes_by_b3_only
storage_anchor_artifact_report_rejects_authority_flags_unknown_fields_and_bad_b3
storage_anchor_artifact_report_exposes_no_payment_or_unlock_authority_keys
storage_phase5_docs_record_anchor_artifact_non_authority_boundary
storage_runtime_source_does_not_gain_phase5_anchor_authority
```

The focused Phase 5 storage test passed after scanner narrowing:

```text
running 5 tests
5 passed
```



## Boundary enforced

The test-local artifact report proves these flags stay safe:

```text
byte_identity_only = true
report_only = true
evidence_only = true

storage_side_effect = false
wallet_side_effect = false
ledger_side_effect = false
balance_truth = false
payment_truth = false
paid_unlock_authority = false
settlement_truth = false
reward_truth = false
outside_chain_truth = false
```

## Authority fields rejected

The test rejects unknown/poison authority fields such as:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payment_receipt
paid_unlock
cache_unlock
settlement_status
finalized
reward_payout
bridge_settlement
solana
rox
```

## Scanner fix applied

The first storage run failed because the scanner searched for bare:

```text
solana
```

That was too broad because `src/config.rs` already contained a negative/deny-test spelling like:

```text
settle-on-solana
```

The fix narrowed the scanner to authority-shaped tokens:

```text
solana_runtime
solana_settlement
solana_anchor_authority
rox_runtime
rox_settlement
```

After that fix, the full storage Phase 5 test and parking gate passed. 

## Runtime source scanner

The new scanner verifies `svc-storage/src` does not gain Phase 5 runtime authority patterns such as:

```text
anchor_paid_unlock
paid_unlock_from_anchor
cache_unlock_from_anchor
settle_from_anchor
commit_from_anchor
apply_anchor
anchor_wallet_receipt
anchor_ledger_receipt
wallet_side_effect: true
ledger_side_effect: true
balance_truth: true
payment_truth: true
paid_unlock_authority: true
settlement_truth: true
reward_truth: true
outside_chain_truth: true
bridge_settlement
external_settlement
solana_runtime
solana_settlement
solana_anchor_authority
rox_runtime
rox_settlement
liquidity_pool
```

## What svc-storage still cannot do

```text
No anchor-based paid unlock.
No cache-only paid unlock.
No anchor-created payment truth.
No anchor-created balance truth.
No anchor-created reward truth.
No wallet mutation.
No ledger mutation.
No fake receipts.
No fake balances.
No settlement truth.
No outside-chain ROC truth.
No bridge.
No ROX runtime.
No Solana runtime.
No staking.
No liquidity.
```

## Green gate summary

```text
Focused Phase 5 test: 5/5 passed
Focused QuickChain tests discovered: 22
All focused QuickChain tests passed
All-targets test passed
Clippy passed
Parking gate passed
```

---

# Pair-level summary

This crate pair now proves the Phase 5 Round 1 anchor-only boundary for reward planning and storage artifacts:

```text
svc-rewarder:
  reward manifest commitments may be referenced as anchor evidence only.

svc-storage:
  opaque anchor dry-run artifact bytes may be stored and retrieved by b3 only.
```

Neither crate becomes economic authority:

```text
No wallet mutation.
No ledger mutation.
No payout execution.
No paid unlock authority.
No balance truth.
No reward truth.
No settlement truth.
No outside-chain ROC truth.
```

No forbidden scope was introduced:

```text
No ROX active runtime.
No Solana active runtime.
No public bridge.
No external settlement.
No staking.
No liquidity.
No exchange-facing logic.
No public-chain authority.
No fake receipts.
No fake balances.
No silent spend.
```

---

# Final status

```text
svc-rewarder + svc-storage
QuickChain Phase 5 Round 1
COMPLETE / PARKED
```

Current Phase 5 Round 1 progress:

```text
ron-proto + ron-ledger: COMPLETE / PARKED
svc-wallet + ron-accounting: COMPLETE / PARKED
svc-rewarder + svc-storage: COMPLETE / PARKED
```

Next crate pair:

```text
svc-gateway + omnigate
```

Next target:

```text
Gateway and omnigate may expose/hydrate anchor dry-run evidence status only.
They must not mutate ledger truth, unlock paid content from anchors, claim external settlement, or turn anchors into ROC balance/finality truth.
```


### END NOTE - JUNE 27 2026 - 19:05 CST


### BEGIN NOTE - JUNE 28 2026 - 00:40 CST

Here are the crate notes for the completed **QuickChain Phase 5 Round 2 `svc-rewarder + svc-storage` pass**. The terminal output confirms both focused DA fallback tests, both parking gates, all-targets tests, clippy, and exhaustive QuickChain preflights passed. 

QuickChain Phase 5 Round 2 Crate Notes — svc-rewarder + svc-storage

Status

COMPLETE / GREEN for QuickChain Phase 5 Round 2 on the svc-rewarder + svc-storage crate pair.

Round theme

Phase 5 Round 2: DA/archive/challenge fallback.

The purpose of this pair was to carry the DA/archive/challenge fallback boundary into reward planning and storage/artifact handling without allowing either crate to become pruning authority, paid-unlock authority, payment truth, reward truth, settlement truth, wallet truth, ledger truth, or outside-chain truth.

Crates covered

svc-rewarder

svc-storage

Terminal result summary

Focused gates passed:

svc-rewarder DA fallback reward boundary test: 5/5 passed

svc-storage DA fallback artifact boundary test: 4/4 passed

Exhaustive gates passed:

svc-rewarder QuickChain preflight passed with 20 focused QuickChain tests discovered.

svc-storage QuickChain preflight passed with 23 focused QuickChain tests discovered.

All-targets tests passed:

svc-rewarder all-targets test passed.

svc-storage all-targets test passed.

Clippy passed:

svc-rewarder clippy passed.

svc-storage clippy passed.

Parking gates passed:

svc-rewarder QuickChain parking gate passed.

svc-storage QuickChain parking gate passed.

Final terminal completion line:

QuickChain Phase 5 Round 2 svc-rewarder shadow fix and svc-rewarder + svc-storage gates completed.

Files added

svc-rewarder:

crates/svc-rewarder/tests/quickchain_phase5_da_fallback_reward_boundary.rs

svc-storage:

crates/svc-storage/tests/quickchain_phase5_da_fallback_artifact_boundary.rs

No production runtime files were changed for this pair.

The patch was intentionally test-only boundary hardening.

svc-rewarder notes

What changed

Added a Phase 5 Round 2 DA/archive/challenge fallback reward boundary test.

The new test defines a strict local evidence-only report shape for rewarder DA fallback context and proves that DA/archive/challenge evidence cannot become reward entitlement, direct payout execution, wallet mutation, ledger mutation, pruning authority, external settlement truth, or outside-chain ROC truth.

New test

quickchain_phase5_da_fallback_reward_boundary

Test count:

5 tests passed

Test coverage

The test proves:

Rewarder DA fallback evidence is report-only.

Rewarder DA fallback evidence is evidence-only.

Archive fallback must be checked.

Missing-data challenge handling must be checked.

Restore path must be checked.

Pruning remains blocked.

The report must reference a nonempty reward manifest.

Canonical b3 hashes are required.

Bad checkpoint hash rejects.

Bad data availability root rejects.

Bad reward manifest commitment rejects.

Bad challenged chunk ID rejects.

Empty payout context rejects.

Unknown authority fields reject.

Authority flags reject when enabled.

Required blocker/check flags reject when disabled.

DA fallback evidence cannot drive a second payout.

DA fallback evidence cannot drive a carrier reward.

DA fallback evidence cannot drive an archive reward.

DA fallback evidence cannot create wallet receipts.

DA fallback evidence cannot create ledger receipts.

DA fallback evidence cannot become reward truth.

Source scanner coverage

The source scanner confirms svc-rewarder does not implement DA fallback reward/runtime authority through forbidden patterns such as:

pay_from_da_fallback

reward_from_da_fallback

issue_from_da_fallback

mint_from_archive_restore

pay_from_missing_data_challenge

direct_carrier_reward

direct_archive_reward

archive_reward_receipt

carrier_reward_receipt

da_fallback_wallet_issue

da_fallback_ledger_commit

pruning authority

outside DA truth

outside chain truth

bridge settlement

external settlement

Solana runtime

ROX runtime

Important svc-rewarder invariant preserved

svc-rewarder remains deterministic payout planning only.

It may produce or reference reward manifests as planning artifacts, but it does not mutate wallet or ledger truth.

DA/archive/challenge fallback evidence can inform review context, but it cannot become payout authority, reward truth, pruning authority, settlement truth, or external-chain truth.

svc-storage notes

What changed

Added a Phase 5 Round 2 DA/archive/challenge fallback artifact boundary test.

The new test proves storage can retain and restore opaque DA fallback artifacts by canonical b3 while remaining byte storage only.

New test

quickchain_phase5_da_fallback_artifact_boundary

Test count:

4 tests passed

Test coverage

The test proves:

DA fallback artifacts can be stored by canonical b3.

DA fallback artifacts can be retrieved by full object read.

DA fallback artifacts can be retrieved by bounded range read.

DA fallback artifacts expose content-derived ETag behavior.

Artifact reports are byte-identity-only.

Artifact reports are report-only.

Artifact reports are evidence-only.

Archive fallback must be checked.

Missing-data challenge handling must be checked.

Restore path must be checked.

Pruning remains blocked.

Canonical b3 hashes are required.

Bad checkpoint hash rejects.

Bad data availability root rejects.

Bad artifact CID rejects.

Bad challenged chunk ID rejects.

Empty artifact evidence rejects.

Unknown authority fields reject.

Authority flags reject when enabled.

Required blocker/check flags reject when disabled.

Stored DA fallback artifacts do not become paid unlock authority.

Stored DA fallback artifacts do not become payment truth.

Stored DA fallback artifacts do not become reward truth.

Stored DA fallback artifacts do not become balance truth.

Stored DA fallback artifacts do not become pruning authority.

Stored DA fallback artifacts do not become settlement truth.

Stored DA fallback artifacts do not become wallet or ledger truth.

Source scanner coverage

The source scanner confirms svc-storage does not construct DA fallback artifact/unlock/pruning authority through forbidden patterns such as:

unlock_from_da_fallback

paid_unlock_from_da_fallback

unlock_from_archive_restore

unlock_from_missing_data_challenge

da_fallback_unlock_authority

archive_restore_payment_truth

missing_data_payment_truth

artifact_payment_truth

artifact_balance_truth

artifact_reward_truth

prune_from_da_fallback

allow_pruning_from_da

pruning authority

outside DA truth

outside chain truth

wallet mutation from archive

ledger mutation from archive

bridge settlement

external settlement

Solana runtime

ROX runtime

Important svc-storage invariant preserved

svc-storage remains bytes/artifacts by canonical b3.

It can store and retrieve DA/archive/challenge artifacts as opaque bytes, but b3 proves byte identity only.

Storage does not become payment truth, paid-unlock authority, reward truth, balance truth, pruning authority, settlement truth, wallet truth, ledger truth, or outside-chain truth.

Bug encountered and fixed

Initial failure

svc-rewarder failed to compile because a local variable named report shadowed the helper function report().

Compiler error:

expected function, found RewarderDaFallbackEvidenceReport

Cause

The test used:

let mut report = report();

After that local binding, later calls to report() tried to call the local variable instead of the helper function.

Fix

Renamed the local variables in the bad-hash/empty-context test to specific names:

bad_checkpoint_report

bad_da_root_report

bad_commitment_report

bad_chunk_report

empty_manifest_report

After the shadowing fix, the focused rewarder DA fallback test passed 5/5 and the full pair parking gates passed.

Architecture result

This crate pair is now aligned with Phase 5 Round 2 doctrine:

DA/archive/challenge fallback evidence remains evidence-only.

Archive fallback checks are present.

Missing-data challenge checks are present.

Restore path checks are present.

Pruning remains blocked.

Rewarder remains payout planning only.

Storage remains bytes/artifacts only.

No direct wallet mutation was introduced.

No direct ledger mutation was introduced.

No paid unlock authority was introduced.

No payment truth was introduced.

No reward truth was introduced.

No balance truth was introduced.

No settlement truth was introduced.

No outside DA truth was introduced.

No outside chain truth was introduced.

No ROX/Solana runtime was introduced.

No public bridge was introduced.

No external settlement path was introduced.

No staking, liquidity, or exchange-facing logic was introduced.

Do not regress

Do not allow svc-rewarder to pay from DA fallback evidence.

Do not allow svc-rewarder to pay from missing-data challenges.

Do not allow svc-rewarder to mint or issue from archive restore material.

Do not allow svc-rewarder to treat carrier/archive evidence as direct ROC entitlement.

Do not allow svc-rewarder to create wallet or ledger receipts.

Do not allow svc-rewarder to bypass svc-wallet.

Do not allow svc-storage artifacts to unlock paid content.

Do not allow svc-storage artifacts to become payment truth.

Do not allow svc-storage artifacts to become reward truth.

Do not allow svc-storage artifacts to become balance truth.

Do not allow svc-storage artifacts to become pruning authority.

Do not allow b3 presence alone to imply paid access.

Do not allow archive restore evidence to mutate wallet or ledger state.

Do not allow missing-data challenge evidence to mutate wallet or ledger state.

Do not add bridge, external settlement, ROX, Solana, staking, liquidity, or exchange-facing logic.

Remaining risks

This pair adds boundary tests, not a full DA/archive restoration runtime.

The actual archive carrier workflow remains future work.

The actual missing-data challenge runtime remains future work.

The actual restore-from-archive path remains future work.

Pruning is still blocked.

Downstream crates still need their Phase 5 Round 2 passes so gateway, omnigate, index, policy, and CrabLink do not interpret DA fallback artifacts as unlock, finality, settlement, or pruning authority.

Next crate pair

svc-gateway + omnigate

Next pair objective

Carry Phase 5 Round 2 DA/archive/challenge fallback boundaries into the public gateway and hydration/coordinator layer.

Expected direction:

svc-gateway may expose/read/proxy DA fallback evidence only as non-authoritative status or artifact context.

svc-gateway must not treat DA fallback evidence as paid unlock authority.

svc-gateway must not treat archive restore evidence as pruning authority.

svc-gateway must not mutate wallet or ledger state from DA fallback material.

omnigate may hydrate DA fallback artifact/status context, but must not turn it into access truth, payment truth, settlement truth, finality truth, or pruning authority.

omnigate must continue to rely on backend wallet/ledger truth for paid unlocks.

Suggested next focused tests

svc-gateway:

quickchain_phase5_da_fallback_gateway_boundary

omnigate:

quickchain_phase5_da_fallback_omnigate_boundary

Suggested next command pattern

cargo fmt -p svc-gateway -p omnigate

cargo test -p svc-gateway --test quickchain_phase5_da_fallback_gateway_boundary

cargo test -p omnigate --test quickchain_phase5_da_fallback_omnigate_boundary

bash crates/svc-gateway/scripts/dev-quickchain-park.sh

bash crates/omnigate/scripts/dev-quickchain-park.sh


### END NOTE - JUNE 28 2026 - 00:40 CST


### BEGIN NOTE - JUNE 28 2026 - 20:00 CST

Your terminal output confirms both crates are green: `svc-rewarder` focused Phase 5 Round 3 test **5/5**, `svc-storage` focused Phase 5 Round 3 test **6/6**, exhaustive preflight passed with **21 rewarder QuickChain tests** and **24 storage QuickChain tests**, and both park gates passed. 

### BEGIN NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-rewarder

---

# CARRY-OVER NOTES — svc-rewarder QuickChain Phase 5 Round 3

**Date:** 2026-06-28
**Crate:** `svc-rewarder`
**Phase/Round:** QuickChain Phase 5 Round 3
**Status:** Complete / parked green
**Verdict:** `svc-rewarder` QuickChain work is complete for the current buildplan through Phase 5. The crate now has the final Phase 5 Round 3 selected external-posture boundary proving that external posture evidence may reference reward manifest commitments only as report/evidence metadata, but cannot become direct reward eligibility, payout execution, wallet mutation, ledger mutation, reward truth, settlement truth, bridge authority, market authority, liquidity authority, ROX/Solana runtime authority, or outside-program authority.

---

## 0) TL;DR

`svc-rewarder` now has a Phase 5 Round 3 boundary test for the selected external integration posture.

The selected posture remains:

```text
anchor-only
report-only
evidence-only
wallet/ledger truth canonical
```

`svc-rewarder` remains:

```text
deterministic payout planning only
not wallet mutation authority
not ledger mutation authority
not reward truth
not payout execution truth
not external settlement authority
```

External posture evidence can reference a reward manifest commitment, run key, inputs CID, checkpoint commitment, and payout count as evidence. It cannot turn that evidence into protocol ROC allocation, wallet mutation, direct payout execution, paid unlock, market authority, liquidity authority, bridge authority, or outside-chain truth.

---

## 1) Files added or changed

### Added

```text
crates/svc-rewarder/tests/quickchain_phase5_external_posture_reward_boundary.rs
```

### Repaired

```text
crates/svc-rewarder/tests/quickchain_phase5_external_posture_reward_boundary.rs
```

The repair corrected the test constant:

```text
POLICY_HASH = b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
```

The prior hash string was malformed, and `svc-rewarder` correctly rejected it with:

```text
BadRequest("policy_hash must be b3:<64 lowercase hex chars>")
```

After replacement with a valid `b3:<64 lowercase hex>` value, the focused test passed.

---

## 2) New focused test target

```text
cargo test -p svc-rewarder --test quickchain_phase5_external_posture_reward_boundary
```

Final result:

```text
running 5 tests
5 passed
0 failed
```

Test cases:

```text
rewarder_external_posture_report_is_anchor_only_evidence_and_not_reward_authority
rewarder_external_posture_report_rejects_authority_flags_and_unknown_fields
rewarder_compute_request_rejects_external_posture_authority_poison_fields
reward_manifest_and_settlement_batch_do_not_become_external_posture_payout_or_truth
rewarder_source_does_not_construct_external_posture_runtime_authority
```

---

## 3) What the new test proves

The new boundary test defines a local test-only external posture report:

```text
RewarderExternalPostureEvidenceReport
```

Schema:

```text
svc-rewarder.quickchain-external-posture-evidence.v1
```

The report can bind:

```text
chain_id
epoch_id
source
posture_id
chosen_posture
posture_semantics
checkpoint_commitment
reward_manifest_commitment
reward_run_key
reward_inputs_cid
payout_count
produced_at_ms
```

Required true flags:

```text
anchor_only_selected
report_only
evidence_only
wallet_ledger_truth_canonical
```

Required false flags:

```text
direct_reward_eligibility
rewarder_side_effect
wallet_side_effect
ledger_side_effect
payout_side_effect
reward_truth
paid_unlock_authority
settlement_truth
outside_da_truth
outside_chain_truth
outside_program_authority
bridge_authority
exchange_facing_authority
public_market
liquidity_enabled
bonded_economy_authority
```

The report rejects unknown authority fields through `serde(deny_unknown_fields)`.

---

## 4) Rewarder request boundary

The test confirms `ComputeEpochRequest` rejects external posture poison fields such as:

```text
external_posture
chosen_external_posture
direct_reward_eligibility
payout_from_external_posture
wallet_side_effect
ledger_side_effect
reward_truth
paid_unlock_authority
bridge_authority
outside_program_authority
exchange_facing_authority
public_market
liquidity_enabled
```

This keeps Phase 5 Round 3 posture out of the live reward compute request surface.

---

## 5) Reward manifest and wallet handoff boundary

The test confirms reward manifests and settlement batches do not expose external posture authority keys.

They remain normal reward-planning artifacts and wallet-handoff plans.

Forbidden concepts stay absent from manifest/settlement JSON:

```text
external_posture
chosen_external_posture
external_da_selected
external_l2_selected
hybrid_selected
direct_reward_eligibility
payout_from_external_posture
reward_truth
settlement_truth
outside_chain_truth
paid_unlock_authority
bridge_authority
outside_program_authority
exchange_facing_authority
public_market
liquidity_enabled
rox_runtime
solana_runtime
```

---

## 6) Source authority scan

The new source scanner confirms `svc-rewarder/src` does not construct Phase 5 Round 3 external posture runtime authority.

It blocks source-level drift toward:

```text
external_posture_payout
payout_from_external_posture
reward_from_external_posture
settle_from_external_posture
commit_from_external_posture
apply_external_posture
external_posture_wallet_receipt
external_posture_ledger_receipt
paid_unlock_from_external_posture
direct_reward_eligibility: true
rewarder_side_effect: true
wallet_side_effect: true
ledger_side_effect: true
payout_side_effect: true
reward_truth: true
settlement_truth: true
outside_da_truth: true
outside_chain_truth: true
outside_program_authority: true
bridge_authority: true
exchange_facing_authority: true
public_market: true
liquidity_enabled: true
bonded_economy_authority: true
external_da_selected: true
external_l2_selected: true
hybrid_selected: true
solana_runtime
rox_runtime
bridge_settlement
exchange_facing
liquidity_pool
```

---

## 7) Preflight and park status

`svc-rewarder` dynamic QuickChain preflight discovered:

```text
21 focused QuickChain tests
```

The new test was discovered automatically because the crate preflight scans:

```text
crates/svc-rewarder/tests/quickchain*.rs
```

No preflight script edit was required.

Park result:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=21
svc-rewarder QuickChain parking gate passed
```

All-targets result:

```text
svc-rewarder all-targets test: passed
```

Clippy result:

```text
cargo clippy -p svc-rewarder --all-targets -- -D warnings
passed
```

---

## 8) Doctrine preserved

`svc-rewarder` remains aligned with the QuickChain doctrine:

```text
rewarder plans payouts
rewarder does not mutate wallet
rewarder does not mutate ledger
rewarder does not create balances
rewarder does not create receipts
rewarder does not create finality
rewarder does not create roots
rewarder does not execute external settlement
rewarder does not turn raw engagement into direct protocol ROC
```

The economic truth path remains:

```text
ron-proto DTOs
→ ron-ledger durable truth
→ svc-wallet mutation front-door
→ ron-accounting snapshots/reports
→ svc-rewarder payout planning only
```

---

## 9) Completion status

For `svc-rewarder`:

```text
QuickChain Phase 0: complete
QuickChain Phase 1: complete
QuickChain Phase 2: complete
QuickChain Phase 3: complete
QuickChain Phase 4: complete
QuickChain Phase 5 Round 1: complete
QuickChain Phase 5 Round 2: complete
QuickChain Phase 5 Round 3: complete

Current planned QuickChain work: complete / parked green
```

---

## 10) Future caution

Do not add any of the following to `svc-rewarder`:

```text
external posture payout execution
external posture reward eligibility
direct wallet mutation
direct ledger mutation
external settlement
public bridge
ROX runtime
Solana runtime
staking
liquidity
exchange-facing logic
outside-program authority
paid unlock authority
reward truth from external evidence
```

`svc-rewarder` can continue to produce deterministic payout plans and evidence artifacts, but only `svc-wallet` may mutate wallet state, and only `ron-ledger` remains durable economic truth.

---

### END NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-rewarder

### BEGIN NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-storage

---

# CARRY-OVER NOTES — svc-storage QuickChain Phase 5 Round 3

**Date:** 2026-06-28
**Crate:** `svc-storage`
**Phase/Round:** QuickChain Phase 5 Round 3
**Status:** Complete / parked green
**Verdict:** `svc-storage` QuickChain work is complete for the current buildplan through Phase 5. The crate now has the final Phase 5 Round 3 selected external-posture artifact boundary proving that external posture artifacts may be stored and retrieved only as opaque canonical `b3` bytes. Those bytes cannot become paid unlock authority, payment truth, wallet truth, ledger truth, reward truth, settlement truth, pruning authority, bridge authority, market authority, liquidity authority, ROX/Solana runtime authority, or outside-program authority.

---

## 0) TL;DR

`svc-storage` now has a Phase 5 Round 3 external posture artifact boundary.

The selected posture remains:

```text
anchor-only
artifact-reference-only
evidence-only
byte-identity-only
wallet/ledger truth canonical
```

`svc-storage` remains:

```text
bytes by canonical b3 only
not wallet truth
not ledger truth
not payment truth
not reward truth
not settlement truth
not paid unlock authority
not pruning authority
not bridge authority
not external settlement authority
```

External posture artifacts can be stored, retrieved, ranged, and verified by canonical `b3`. They cannot unlock paid content or mutate economic state.

---

## 1) Files added or changed

### Added

```text
crates/svc-storage/tests/quickchain_phase5_external_posture_artifact_boundary.rs
```

No production source changes were required.

No preflight script edit was required because `svc-storage` dynamically discovers `quickchain*.rs` tests.

---

## 2) New focused test target

```text
cargo test -p svc-storage --test quickchain_phase5_external_posture_artifact_boundary
```

Final result:

```text
running 6 tests
6 passed
0 failed
```

Test cases:

```text
storage_external_posture_artifact_report_is_anchor_only_byte_reference_only
storage_external_posture_artifact_report_rejects_authority_flags_and_unknown_fields
external_posture_artifact_bytes_store_by_b3_without_unlock_or_payment_authority
storage_accounting_export_rejects_external_posture_authority_fields
storage_usage_event_export_remains_metering_not_external_posture_authority
storage_source_does_not_construct_external_posture_runtime_or_unlock_authority
```

---

## 3) What the new test proves

The test defines a local test-only artifact report:

```text
StorageExternalPostureArtifactReport
```

Schema:

```text
svc-storage.quickchain-external-posture-artifact.v1
```

The report can bind:

```text
chain_id
artifact_cid
checkpoint_commitment
artifact_kind
chosen_posture
posture_semantics
source
produced_at_ms
```

Required true flags:

```text
anchor_only_selected
artifact_reference_only
evidence_only
byte_identity_only
wallet_ledger_truth_canonical
```

Required false flags:

```text
storage_side_effect
paid_unlock_authority
payment_truth
wallet_truth
ledger_truth
reward_truth
settlement_truth
outside_da_truth
outside_chain_truth
outside_program_authority
bridge_authority
exchange_facing_authority
public_market
liquidity_enabled
pruning_authority
bonded_economy_authority
```

The report rejects unknown authority fields through `serde(deny_unknown_fields)`.

---

## 4) b3 byte-storage boundary

The test stores external posture artifact bytes in `MemoryStorage`.

It verifies:

```text
CID is derived from the bytes
CID is canonical b3:<64 lowercase hex>
stored artifact is discoverable only by exact b3
HEAD returns exact length and matching ETag
GET full returns the original bytes
GET range returns bounded bytes only
```

This proves the storage path treats external posture artifacts as byte-addressed objects only.

Important boundary:

```text
b3 proves byte identity only
b3 does not prove paid access
b3 does not prove wallet truth
b3 does not prove ledger truth
b3 does not prove settlement truth
b3 does not prove pruning authority
```

---

## 5) Accounting export boundary

The test confirms `AccountingExportRequest` and nested `UsageEventDto` reject external posture authority poison fields.

Rejected top-level and nested fields include:

```text
external_posture
chosen_external_posture
paid_unlock_authority
payment_truth
wallet_truth
ledger_truth
reward_truth
settlement_truth
outside_da_truth
outside_chain_truth
outside_program_authority
bridge_authority
exchange_facing_authority
public_market
liquidity_enabled
pruning_authority
```

This preserves the accounting boundary:

```text
usage events are metering only
accounting export is not balance truth
accounting export is not paid unlock authority
accounting export is not settlement authority
```

---

## 6) Usage event export boundary

The test confirms storage usage/accounting export remains ordinary metering:

```text
metric_kind = bytes_stored
source_service = svc-storage
```

The serialized accounting export does not expose:

```text
external_posture
chosen_external_posture
paid_unlock_authority
payment_truth
wallet_truth
ledger_truth
reward_truth
settlement_truth
outside_chain_truth
bridge_authority
outside_program_authority
exchange_facing_authority
public_market
liquidity_enabled
pruning_authority
```

This keeps storage metering distinct from economic authority.

---

## 7) Source authority scan

The new source scanner confirms `svc-storage/src` does not construct external posture runtime or unlock authority.

It blocks source-level drift toward:

```text
external_posture_paid_unlock
paid_unlock_from_external_posture
unlock_from_external_posture
unlock_from_posture_artifact
external_posture_wallet_receipt
external_posture_payment_truth
external_posture_balance_truth
external_posture_settlement_truth
cache_external_posture_authority
cache_unlock_authority_from_posture
storage_unlock_authority_from_posture
outside_da_truth: true
outside_chain_truth: true
outside_program_authority: true
bridge_authority: true
exchange_facing_authority: true
public_market: true
liquidity_enabled: true
pruning_authority: true
bonded_economy_authority: true
solana_runtime
rox_runtime
bridge_settlement
exchange_facing
liquidity_pool
```

---

## 8) Preflight and park status

`svc-storage` dynamic QuickChain preflight discovered:

```text
24 focused QuickChain tests
```

The new test was discovered automatically because the crate preflight scans:

```text
crates/svc-storage/tests/quickchain*.rs
```

Park result:

```text
svc-storage quickchain exhaustive preflight gate passed: tests=24
svc-storage QuickChain parking gate passed
```

All-targets result:

```text
svc-storage all-targets test: passed
```

Clippy result:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings
passed
```

---

## 9) Doctrine preserved

`svc-storage` remains aligned with QuickChain and CrabLink/Tauri doctrine:

```text
storage stores bytes by b3
cache is convenience only
cache is not paid-access authority
storage is not wallet truth
storage is not ledger truth
storage is not reward truth
storage is not settlement truth
storage is not finality truth
storage artifacts are not roots
storage artifacts are not validator authority
storage artifacts are not bridge authority
storage artifacts are not pruning authority
```

Paid storage remains gated by backend-derived wallet proof paths, not cache or artifact presence alone.

---

## 10) Completion status

For `svc-storage`:

```text
QuickChain Phase 0: complete
QuickChain Phase 1: complete
QuickChain Phase 2: complete
QuickChain Phase 3: complete
QuickChain Phase 4: complete
QuickChain Phase 5 Round 1: complete
QuickChain Phase 5 Round 2: complete
QuickChain Phase 5 Round 3: complete

Current planned QuickChain work: complete / parked green
```

---

## 11) Future caution

Do not add any of the following to `svc-storage`:

```text
external posture paid unlock
unlock from external posture artifact
cache-based paid unlock authority
payment truth from artifact bytes
wallet truth from artifact bytes
ledger truth from artifact bytes
reward truth from artifact bytes
settlement truth from artifact bytes
external settlement
public bridge
ROX runtime
Solana runtime
staking
liquidity
exchange-facing logic
outside-program authority
pruning authority before DA/challenge/archive fallback is proven and authorized
```

`svc-storage` can store and serve canonical `b3` artifacts, but backend wallet/ledger paths remain the economic truth.

---

## 12) Next crate-pair handoff

Next crate pair:

```text
svc-gateway + omnigate
```

Expected Phase 5 Round 3 posture there:

```text
svc-gateway:
  selected external posture may be exposed only as evidence/status metadata.
  it must not become paid unlock authority.
  it must not mutate wallet or ledger truth.
  it must not create bridge, settlement, market, liquidity, ROX/Solana, or outside-program authority.

omnigate:
  selected external posture may hydrate/read evidence only.
  it must not become content unlock authority.
  it must not bypass paid gates.
  it must not mutate wallet or ledger truth.
  it must not treat external evidence as finality, balance, receipt, or settlement truth.
```

---

### END NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-storage


### END NOTE - JUNE 28 2026 - 20:00 CST


### BEGIN NOTE - JUNE 29 2026 - 13:30 CST

The terminal output confirms both focused tests passed, then both crate-local park gates passed: `svc-rewarder` parked with **21 QuickChain tests**, and `ron-policy` parked with **23 QuickChain tests**. 

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-rewarder

## Status

`svc-rewarder` is GREEN / PARKED for Internal ROC Beta Phase 1 Round 1 on the reward-planning non-authority slice.

This was the third completed crate-pair pass in the active Internal ROC Beta flow:

```text
1. ron-proto + ron-ledger          GREEN / PARKED
2. svc-wallet + ron-accounting     GREEN / PARKED
3. svc-rewarder + ron-policy       GREEN / PARKED
4. svc-storage + svc-index         NEXT
5. svc-gateway + omnigate          pending
6. CrabLink Tauri/client adapters  pending
```

This does not mean all of Phase 1 is complete.

It means the `svc-rewarder` side of the `svc-rewarder + ron-policy` crate pair is complete for this planning/policy non-authority proof slice.

## Round purpose

Internal ROC Beta Phase 1 is proving paid post, paid comment, paid article, and content_view support while preserving the internal ROC authority model.

For `svc-rewarder`, the goal was to prove that reward planning remains deterministic, capped, and non-authoritative.

`svc-rewarder` may produce plans and wallet handoff material.

`svc-rewarder` must not create receipt truth, balance truth, unlock truth, payout execution truth, finality truth, wallet authority, or ledger authority.

## File added

```text
crates/svc-rewarder/tests/internal_roc_beta_rewarder_planning_non_authority.rs
```

## Follow-up fix applied

The first focused run exposed one test assertion issue.

The original assertion treated a wallet issue idempotency key as if it had to be a canonical `b3:<64hex>` content/commitment hash.

Actual behavior:

```text
b3:<60 lowercase hex chars>
```

That is acceptable because this field is bounded retry/dedupe material, not canonical receipt, operation, root, or content-hash authority.

Fix applied:

```text
assert_bounded_b3_idempotency_key(...)
```

New assertion rule:

```text
idempotency key must:
  - start with b3:
  - have lowercase hex body
  - have bounded length 32..=64
  - remain retry/dedupe material only
```

This correction is architecturally correct.

`run_key` and `manifest.commitment` remain canonical `b3:<64hex>` commitments.

Wallet issue idempotency keys remain bounded retry/dedupe keys, not operation authority.

## Focused test added

```bash
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
```

## Focused test result

```text
running 5 tests

protocol_pool_planning_requires_signed_policy_and_stays_provenance_only ... ok
paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth ... ok
wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt ... ok
reward_policy_rejects_paid_content_authority_poison_fields ... ok
compute_request_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

## Parking / preflight result

The crate-local parking gate passed.

Terminal proof:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=21 ==
== svc-rewarder QuickChain parking gate passed ==
```

The park gate also ran:

```text
forbidden helper scan
cargo fmt check
21 focused QuickChain tests
svc-rewarder all-targets test
svc-rewarder clippy
bench smoke for reward_calc_100
forbidden-scope marker
```

No checked-in Python helpers were found under `crates/svc-rewarder`.

Clippy completed cleanly.

## Focused proof details

The new test proves:

```text
reward planning is deterministic
dry-run planning emits no wallet/ledger effects
reward manifests are planning artifacts only
manifest commitment is reference material only
settlement batches are handoff material only
wallet issue batches are request previews only
wallet issue batches are not receipts
wallet issue batches are not balance truth
wallet issue batches are not payout execution truth
wallet issue idempotency keys are retry/dedupe material only
protocol_pool planning requires signed policy
RewardPolicy rejects authority poison fields
ComputeEpochRequest rejects authority poison fields
```

## Paid-content coverage

The reward-planning test used Internal ROC beta paid-content style contributors:

```text
acct_post_creator
acct_comment_creator
acct_article_creator
acct_content_view_creator
```

This proves the rewarder can deterministically plan over the paid-content beta surface while remaining downstream planning infrastructure only.

## Authority poison rejected

The focused tests reject smuggled fields such as:

```text
wallet_mutation
ledger_mutation
balance_truth
receipt_truth
paid_unlock_authority
entitlement_truth
payout_execution_truth
client_finality_claim
cache_unlock_authority
gateway_receipt_truth
omnigate_receipt_truth
bridge_txid
staking_position_id
liquidity_pool_id
external_settlement_id
raw_engagement_mints_roc
```

## Boundaries preserved

`svc-rewarder` remains deterministic payout planning only.

Allowed:

```text
consume accounting-style snapshots
apply signed reward policy
compute deterministic reward manifests
produce capped payout plans
produce settlement batch handoff shapes
produce wallet issue request preview material
use idempotency keys for retry/dedupe
emit planning provenance
```

Forbidden and still not implemented:

```text
wallet mutation
ledger mutation
direct ROC issue
direct ROC transfer
direct ROC burn
hold open
hold capture
hold release
receipt creation
balance truth
paid unlock truth
entitlement truth
payout execution truth
root authority
checkpoint authority
validator authority
bridge runtime
staking runtime
liquidity runtime
external settlement
exchange-facing logic
raw engagement direct minting
```

## Existing QuickChain boundary stayed green

The existing QuickChain suites confirmed `svc-rewarder` still has:

```text
no roots
no checkpoints
no validators
no settlement
no anchors
no bridges
no staking
no liquidity
no direct mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from rewarder outputs
```

The terminal explicitly preserved the marker:

```text
svc-rewarder remains deterministic payout planning only;
svc-wallet remains mutation front-door;
ron-ledger remains truth
```

## Architecture result

This crate now has direct Internal ROC Beta proof that `svc-rewarder` can participate in the paid-content value loop without becoming economic authority.

Correct flow remains:

```text
ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved mutation
→ ron-ledger durable receipt/balance truth
```

Incorrect flow remains blocked:

```text
svc-rewarder plan
→ direct balance mutation
→ direct receipt truth
→ direct paid unlock
```

## Final svc-rewarder status

```text
Internal ROC Beta Phase 1 Round 1
svc-rewarder
GREEN / PARKED
```

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-rewarder

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - ron-policy

## Status

`ron-policy` is GREEN / PARKED for Internal ROC Beta Phase 1 Round 1 on the paid-content policy/economics non-authority slice.

This was the third completed crate-pair pass in the active Internal ROC Beta flow:

```text
1. ron-proto + ron-ledger          GREEN / PARKED
2. svc-wallet + ron-accounting     GREEN / PARKED
3. svc-rewarder + ron-policy       GREEN / PARKED
4. svc-storage + svc-index         NEXT
5. svc-gateway + omnigate          pending
6. CrabLink Tauri/client adapters  pending
```

This does not mean all of Phase 1 is complete.

It means the `ron-policy` side of the `svc-rewarder + ron-policy` crate pair is complete for this planning/policy non-authority proof slice.

## Round purpose

Internal ROC Beta Phase 1 is proving paid post, paid comment, paid article, and content_view support while preserving the internal ROC authority model.

For `ron-policy`, the goal was to prove that policy and economics config may gate and price paid-content behavior without becoming receipt truth, balance truth, entitlement truth, unlock truth, payout truth, wallet authority, or ledger authority.

## File added

```text
crates/ron-policy/tests/internal_roc_beta_paid_content_policy_non_authority.rs
```

## Focused test added

```bash
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
```

## Focused test result

```text
running 5 tests

policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority ... ok
policy_allow_after_backend_context_is_not_paid_unlock_or_receipt_truth ... ok
policy_rejects_paid_content_authority_shaped_tags ... ok
economics_paid_content_view_prices_and_validates_capture_plan_without_authority ... ok
economics_config_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

## Parking / preflight result

The crate-local parking gate passed.

Terminal proof:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=23 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

The park gate also ran:

```text
cargo fmt
23 focused QuickChain tests
existing ron-policy regressions
all-targets tests
clippy
bench smoke for eval:get/us
examples compile checks
```

Clippy completed cleanly.

## Focused proof details

The new test proves:

```text
policy allow decisions are not paid unlock authority
policy allow decisions are not receipt truth
policy allow decisions are not balance truth
policy obligations cannot create payment authority
policy obligations cannot smuggle paid unlock authority
authority-shaped condition tags reject
authority-shaped obligation kinds reject
authority-shaped obligation params reject
paid_content_view economics can price and validate capture plans
economics config remains validation/gating input only
economics config rejects authority poison fields
```

## Safe policy behavior proven

The allowed policy shape can express:

```text
backend-proof-checked
paid-content-policy-context
content-kind-post
require-backend-wallet-ledger-proof
```

This is safe because those are declarative gating/context labels.

They do not become receipt truth.

They do not unlock paid content by themselves.

They do not mutate wallet or ledger.

They only say that gateway/omnigate/backend must depend on backend wallet/ledger proof.

## Authority-shaped tags rejected

The focused test rejects policy condition tags such as:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

## Authority-shaped obligations rejected

The focused test rejects policy obligation kinds such as:

```text
unlock_paid_content
create_receipt
accept_receipt
verify_payment
mutate_balance
credit_account
open_hold
capture_hold
release_hold
settlement_complete
bridge_settlement
```

## Economics config proof

The focused test loads the checked-in ROC economics config and proves:

```text
paid_content_view exists
paid_content_view is enabled
paid_content_view has deterministic positive pricing
paid_content_view split basis points sum exactly to 10000
capture plan validation works as config/gating input
economics JSON shape has no authority fields
```

This confirms that economics config can support paid content pricing/capture validation without becoming payment truth.

## Economics poison rejected

The focused test rejects appended config poison fields such as:

```text
receipt_truth
balance_truth
paid_unlock_authority
ledger_mutation
wallet_mutation
bridge_txid
staking_position_id
liquidity_pool_id
external_settlement_id
```

## Boundaries preserved

`ron-policy` remains declarative policy/economics validation only.

Allowed:

```text
parse policy bundles
evaluate declarative allow/deny rules
validate condition tags
validate obligations
validate economics config
price paid_content_view from config
validate capture plan shape
reject malformed or authority-shaped inputs
act as gating input for gateway/omnigate/backend
```

Forbidden and still not implemented:

```text
wallet mutation
ledger mutation
direct ROC issue
direct ROC transfer
direct ROC burn
hold open
hold capture
hold release
receipt creation
balance truth
paid unlock truth
entitlement truth
payout execution truth
root authority
checkpoint authority
validator authority
bridge runtime
staking runtime
liquidity runtime
external settlement
exchange-facing logic
raw engagement direct minting
```

## Existing QuickChain boundary stayed green

The existing QuickChain suites confirmed `ron-policy` still has:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from policy decisions
```

The terminal explicitly preserved the marker:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

## Architecture result

This crate now has direct Internal ROC Beta proof that `ron-policy` can participate in paid-content pricing and gating without becoming economic authority.

Correct flow remains:

```text
paid-content context
→ ron-policy declarative gating/economics validation
→ backend wallet/ledger proof requirement
→ svc-wallet mutation only when approved
→ ron-ledger durable receipt/balance truth
```

Incorrect flow remains blocked:

```text
policy allow
→ paid unlock
→ receipt truth
→ balance truth
→ wallet mutation
```

## Final ron-policy status

```text
Internal ROC Beta Phase 1 Round 1
ron-policy
GREEN / PARKED
```

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - ron-policy


### END NOTE - JUNE 29 2026 - 13:30 CST


### BEGIN NOTE - JUNE 29 2026 - 18:30 CST

The terminal output confirms both focused Phase 3 preflights passed for `svc-rewarder` and `ron-policy`, including the new reward-plan and policy-gate tests, prior regressions, and clippy.  The buildplan scope for this pair was exactly Round 1 non-mutating payout plans: rewarder consumes sealed snapshots, produces deterministic capped plans, policy validates/gates, no payout execution yet, no rewarder ledger mutation, and no policy-created receipt. 

Below are paste-ready crate notes.

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

# svc-rewarder — Internal ROC Beta Phase 3 Round 1 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 1
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 slice

---

## 0. Safe status label

```text
Internal ROC Beta Phase 3 Round 1 svc-rewarder reward-plan boundary is GREEN / PARKED.
```

Pair label:

```text
Internal ROC Beta Phase 3 Round 1 svc-rewarder + ron-policy reward-plan/policy-gate boundary is GREEN / PARKED.
```

Round-level label:

```text
Internal ROC Beta Phase 3 Round 1 snapshots and non-mutating payout plans are GREEN / PARKED across the intended crate pairs.
```

This does **not** mean all of Phase 3 is complete.

Correct larger status:

```text
QuickChain boundary/preflight scope is COMPLETE / GREEN / PARKED through Phase 5.
Internal ROC Beta Phase 0 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 2 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 2 is NEXT.
```

Still deferred / not authorized:

```text
ROX
Solana
public bridge
external settlement
staking runtime
liquidity
exchange-facing logic
public validator economy
public chain runtime
```

---

## 1. What this slice added

This slice added focused Internal ROC Beta Phase 3 Round 1 coverage for reward planning.

New test:

```text
crates/svc-rewarder/tests/internal_roc_beta_phase3_reward_plan_boundary.rs
```

New script:

```text
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text
crates/svc-rewarder/tests/internal_roc_beta_phase3_reward_plan_boundary.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 test target contains five tests:

```text
reward_plan_is_deterministic_for_same_snapshot_regardless_input_order
reward_plan_enforces_pool_cap_and_conservation_without_execution
wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth
reward_policy_and_compute_request_reject_authority_poison_fields
raw_engagement_fields_cannot_be_rewarder_payout_input_authority
```

These tests prove `svc-rewarder` can create deterministic capped planning material, but cannot become wallet/ledger truth.

---

## 4. Boundary doctrine proven

`svc-rewarder` remains:

```text
deterministic payout planner
accounting snapshot consumer
policy/config input consumer
reward manifest producer
wallet handoff preview producer
non-mutating planning service
```

`svc-rewarder` does **not** become:

```text
wallet mutation authority
ledger mutation authority
receipt truth
balance truth
payout execution truth
finality truth
bridge authority
staking authority
liquidity authority
external settlement authority
public-chain authority
```

Correct Phase 3 Round 1 interpretation:

```text
Accounting snapshot
→ rewarder deterministic capped reward plan
→ policy validation/gating
→ no wallet mutation yet
```

Round 2 is where approved payout execution begins, and only through `svc-wallet`.

---

## 5. Deterministic reward-plan behavior proven

The test confirms the same snapshot material produces the same plan even when contribution input order changes.

Proven stable outputs:

```text
run_key
commitment
inputs_cid
totals
payouts
account ordering
```

The plan output remains deterministic over normalized/canonicalized snapshot input.

The payout accounts are sorted deterministically.

---

## 6. Pool cap and conservation behavior proven

The test confirms `max_payout_minor_units` caps the snapshot pool.

Proven invariant:

```text
payout_minor_units + residual_minor_units == capped pool_minor_units
```

Also proven:

```text
planned payouts never exceed capped pool
residual is explicit
floor rounding remains deterministic
no wallet/ledger effect is emitted
```

This keeps reward planning bounded and prevents planning artifacts from becoming inflation.

---

## 7. Wallet issue batch doctrine preserved

The test confirms `SettlementBatch` and `WalletIssueBatch` are handoff previews only.

Allowed:

```text
wallet_path reference
funding_source reference
deterministic wallet issue request shape
deterministic idempotency key references
asset = roc
integer minor-unit amounts
```

Forbidden:

```text
receipt_hash
balance_minor
settlement_status
finality
operation_id
ledger_mutation
wallet_mutation
```

The wallet batch is a preview/handoff shape, not payout execution and not receipt truth.

Correct interpretation:

```text
RewardManifest = planning artifact
SettlementBatch = handoff preview
WalletIssueBatch = wallet request preview
svc-wallet = approved execution boundary
ron-ledger = durable receipt/balance truth
```

---

## 8. Raw engagement poisoning rejected

The test confirms raw engagement fields cannot be smuggled into rewarder payout input authority.

Rejected root-level fields include:

```text
event_class
source_event_class
raw_views
raw_likes
raw_comments
raw_watch_seconds
analytics_only
metering
proof_eligible
ad_budgeted
reward_material
raw_engagement_mints_roc
```

Rejected contribution-level fields include:

```text
event_class
source_event_class
raw_views
raw_likes
raw_comments
raw_watch_seconds
analytics_only
proof_eligible
ad_budgeted
wallet_receipt
ledger_receipt
```

This preserves the hard doctrine:

```text
Raw engagement never directly mints or allocates protocol ROC.
```

---

## 9. Authority poisoning rejected

The test rejects reward policy and compute request smuggling fields, including:

```text
balance
balance_minor
available_balance
wallet_balance
ledger_balance
wallet_receipt
ledger_receipt
receipt_id
receipt_hash
receipt_root
receipt_txid
payout_receipt_txid
settlement_status
finality
finalized
wallet_mutation
ledger_mutation
ledger_side_effect
wallet_side_effect
payout_execution
payout_execution_truth
operation_id
account_sequence
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_txid
solana_signature
rox_settlement_id
staking_position_id
staking_yield_bps
liquidity_pool_id
exchange_order_id
outside_settlement_claim
```

These fields cannot turn rewarder input/output into receipt, balance, payout, finality, bridge, staking, liquidity, or external settlement truth.

---

## 10. Tests / gates passed

Focused Phase 3 reward-plan boundary test:

```bash
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
```

Result:

```text
running 5 tests
test reward_plan_enforces_pool_cap_and_conservation_without_execution ... ok
test reward_plan_is_deterministic_for_same_snapshot_regardless_input_order ... ok
test raw_engagement_fields_cannot_be_rewarder_payout_input_authority ... ok
test wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth ... ok
test reward_policy_and_compute_request_reject_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC rewarder planning non-authority regression:

```bash
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
```

Result:

```text
running 5 tests
test protocol_pool_planning_requires_signed_policy_and_stays_provenance_only ... ok
test paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth ... ok
test wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt ... ok
test reward_policy_rejects_paid_content_authority_poison_fields ... ok
test compute_request_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain no-direct-mutation regression:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
```

Result:

```text
running 4 tests
test compute_request_still_rejects_direct_mutation_authority_smuggling ... ok
test planning_outputs_do_not_claim_receipts_balances_operation_truth_roots_or_finality ... ok
test config_rejects_external_settlement_bridge_anchor_validator_and_root_knobs ... ok
test router_does_not_expose_direct_wallet_ledger_quickchain_or_bridge_mutation_routes ... ok

test result: ok. 4 passed; 0 failed
```

QuickChain funding-source regression:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
```

Result:

```text
running 6 tests
test policy_requires_explicit_funding_source_on_wire ... ok
test unsigned_protocol_pool_policy_is_rejected_by_validator ... ok
test current_policy_accepts_explicit_protocol_pool_and_rejects_smuggled_authority_fields ... ok
test manifest_carries_funding_provenance_but_not_funding_finality ... ok
test wallet_preview_carries_batch_provenance_but_requests_remain_wallet_issue_shape ... ok
test compute_request_rejects_top_level_funding_authority_smuggling ... ok

test result: ok. 6 passed; 0 failed
```

QuickChain replay/no-double-issue regression:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
```

Result:

```text
running 4 tests
test idempotency_keys_are_retry_dedupe_not_operation_identity ... ok
test duplicate_epoch_replay_is_dedupe_not_second_payout_authority ... ok
test reordered_snapshot_rows_produce_same_plan ... ok
test same_snapshot_policy_and_epoch_produce_same_plan_commitment ... ok

test result: ok. 4 passed; 0 failed
```

Strict Clippy gate:

```bash
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

Result:

```text
Finished `dev` profile
```

Focused Phase 3 preflight:

```bash
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text
== Internal ROC Beta Phase 3 svc-rewarder reward-plan preflight passed ==
== rewarder remains deterministic capped planning only; no wallet/ledger mutation, receipt/balance/finality truth, raw-engagement direct ROC allocation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`svc-rewarder` now proves for Phase 3 Round 1:

```text
reward plans are deterministic.
reward plans are capped.
reward plans conserve capped pool value.
reward plans sort payout accounts deterministically.
reward manifests are planning artifacts only.
settlement batches are handoff previews only.
wallet issue batches are not payout receipts.
wallet issue batches are not balance truth.
raw engagement cannot directly become rewarder payout authority.
rewarder cannot mutate wallet.
rewarder cannot mutate ledger.
rewarder cannot create receipts.
rewarder cannot create balances.
rewarder cannot create finality.
rewarder cannot create bridge/staking/liquidity/external settlement truth.
```

---

## 12. What this does not do yet

This Round 1 slice does **not** execute approved payouts.

Not yet complete:

```text
approved payout execution through svc-wallet
durable payout receipts in ron-ledger
duplicate payout prevention across the full execution path
payout receipt replay/conservation proof
accounting observation of executed payout receipts
Phase 3 overall completion
```

Those belong to Phase 3 Round 2.

---

## 13. Commands to rerun

From repo root:

```bash
cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo clippy -p svc-rewarder --all-targets -- -D warnings
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Phase 3 Round 1 completion context

Phase 3 Round 1 intended crate pairs are now green:

```text
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

Round 1 proved:

```text
accounting/reward-plan reference DTOs are non-authority.
ledger rejects reward-plan material as receipt truth.
accounting snapshots are deterministic derivative material.
wallet/accounting observer boundaries are preserved.
rewarder produces deterministic capped payout plans.
policy validates/gates reward plans declaratively.
no payout execution occurs yet.
```

---

## 15. Next phase context

Next active target:

```text
Internal ROC Beta Phase 3 Round 2 — approved payout execution through svc-wallet
```

Expected Round 2 work involving `svc-rewarder`:

```text
reward plan emits payout intent candidates.
policy validates/gates payout plan.
duplicate payout prevention markers are enforced.
caps and category pools remain enforced.
approved payout intent goes to svc-wallet only.
svc-wallet returns durable receipt.
ron-ledger replay proves payout receipt stability.
```

Do not implement direct rewarder wallet/ledger mutation.

Do not let rewarder become payout execution authority.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

# ron-policy — Internal ROC Beta Phase 3 Round 1 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 1
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 slice

---

## 0. Safe status label

```text
Internal ROC Beta Phase 3 Round 1 ron-policy reward-plan gate boundary is GREEN / PARKED.
```

Pair label:

```text
Internal ROC Beta Phase 3 Round 1 svc-rewarder + ron-policy reward-plan/policy-gate boundary is GREEN / PARKED.
```

Round-level label:

```text
Internal ROC Beta Phase 3 Round 1 snapshots and non-mutating payout plans are GREEN / PARKED across the intended crate pairs.
```

This does **not** mean all of Phase 3 is complete.

Correct larger status:

```text
QuickChain boundary/preflight scope is COMPLETE / GREEN / PARKED through Phase 5.
Internal ROC Beta Phase 0 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 2 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 2 is NEXT.
```

Still deferred / not authorized:

```text
ROX
Solana
public bridge
external settlement
staking runtime
liquidity
exchange-facing logic
public validator economy
public chain runtime
```

---

## 1. What this slice added

This slice added focused Internal ROC Beta Phase 3 Round 1 coverage for declarative reward-plan policy gating.

New test:

```text
crates/ron-policy/tests/internal_roc_beta_phase3_reward_plan_policy_gate.rs
```

New script:

```text
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text
crates/ron-policy/tests/internal_roc_beta_phase3_reward_plan_policy_gate.rs
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 test target contains five tests:

```text
reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth
reward_plan_policy_denial_is_not_refund_receipt_or_balance_truth
known_authority_shaped_reward_plan_policy_tags_reject
known_authority_shaped_reward_plan_obligation_params_reject
reward_plan_policy_gate_remains_declarative_not_execution_surface
```

These tests prove `ron-policy` can gate reward plans, but cannot become payout execution, wallet/ledger mutation, receipt, balance, or finality truth.

---

## 4. Boundary doctrine proven

`ron-policy` remains:

```text
declarative policy engine
reward-plan gate
config validator
eligibility validator
obligation emitter
non-mutating decision surface
```

`ron-policy` does **not** become:

```text
wallet mutation authority
ledger mutation authority
receipt truth
balance truth
payout execution truth
refund truth
finality truth
paid unlock truth
bridge authority
staking authority
liquidity authority
external settlement authority
public-chain authority
```

Correct interpretation:

```text
Policy allow = permission/gate only.
Policy deny = rejection/gate only.
Policy obligation = instruction/requirement only.
Policy config = validation data only.
```

None of these are economic truth.

---

## 5. Policy allow behavior proven

The test confirms a reviewed reward plan can be allowed only when the expected safe tags are present.

Safe tags used:

```text
reward-plan-reviewed
bounded-pool-cap-checked
accounting-snapshot-cid-checked
policy-gate-only
backend-wallet-execution-required
```

A policy allow decision can carry a safe obligation:

```text
require-approved-payout-intent-through-svc-wallet
```

Safe obligation params:

```text
plan_source = svc_rewarder
execution_boundary = svc_wallet
ledger_truth = ron_ledger
```

This obligation does not create receipt, balance, payout, or finality truth.

It only preserves the required execution boundary:

```text
approved payout intent → svc-wallet → ron-ledger receipt
```

---

## 6. Policy deny behavior proven

The test confirms a reward-plan policy denial is not:

```text
refund receipt
wallet receipt
ledger receipt
balance update
payout execution
finality event
paid unlock
```

A deny decision emits no payout/receipt obligations.

Correct interpretation:

```text
Policy deny = gate failure only.
No refund exists until svc-wallet/ron-ledger produce a real backend receipt.
```

---

## 7. Authority-shaped tags rejected

The test confirms known authority-shaped reward-plan policy tags reject.

Rejected examples include:

```text
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents policy tags from becoming proof, receipt, balance, operation identity, or settlement authority.

---

## 8. Authority-shaped obligation params rejected

The test confirms known authority-shaped obligation param keys reject.

Rejected examples include:

```text
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents policy obligations from smuggling economic authority into otherwise declarative decisions.

---

## 9. Declarative gate behavior proven

The test confirms a safe policy gate can require:

```text
reward-plan-reviewed
bounded-pool-cap-checked
policy-gate-only
```

and emit a safe obligation requiring explicit wallet boundary execution.

Still, the policy output remains declarative.

It does not contain authority tokens such as:

```text
receipt_id
receipt_hash
receipt_root
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
bridge_txid
solana_signature
rox_settlement_id
staking_position_id
liquidity_pool_id
operation_id
account_sequence
payout_execution
```

---

## 10. Tests / gates passed

Focused Phase 3 reward-plan policy gate test:

```bash
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
```

Result:

```text
running 5 tests
test reward_plan_policy_denial_is_not_refund_receipt_or_balance_truth ... ok
test known_authority_shaped_reward_plan_obligation_params_reject ... ok
test reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth ... ok
test reward_plan_policy_gate_remains_declarative_not_execution_surface ... ok
test known_authority_shaped_reward_plan_policy_tags_reject ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC paid-content policy non-authority regression:

```bash
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
```

Result:

```text
running 5 tests
test policy_allow_after_backend_context_is_not_paid_unlock_or_receipt_truth ... ok
test policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority ... ok
test policy_rejects_paid_content_authority_shaped_tags ... ok
test economics_paid_content_view_prices_and_validates_capture_plan_without_authority ... ok
test economics_config_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain decision non-authority regression:

```bash
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
```

Result:

```text
running 4 tests
test authority_shaped_obligation_param_key_rejects_policy ... ok
test authority_shaped_obligation_kind_rejects_policy ... ok
test deny_decision_is_still_not_receipt_balance_or_finality_truth ... ok
test allow_decision_is_policy_result_not_paid_unlock_or_receipt_truth ... ok

test result: ok. 4 passed; 0 failed
```

Economics policy regression:

```bash
cargo test -p ron-policy --test economics_policy
```

Result:

```text
running 15 tests
test float_value_rejects_during_parse ... ok
test capture_plan_accepts_required_dynamic_recipient ... ok
test disabled_action_rejects_lookup_but_config_can_load ... ok
test capture_over_action_cap_rejects ... ok
test invalid_split_sum_rejects ... ok
test deterministic_action_order_is_sorted ... ok
test missing_required_action_rejects ... ok
test missing_dynamic_recipient_rejects_capture_plan ... ok
test overflow_value_rejects_during_parse ... ok
test negative_value_rejects_during_parse ... ok
test unknown_action_rejects ... ok
test paid_storage_put_price_uses_minimum_and_hold_multiplier ... ok
test unknown_split_destination_rejects ... ok
test unknown_paid_action_lookup_rejects ... ok
test valid_checked_in_roc_economics_config_loads ... ok

test result: ok. 15 passed; 0 failed
```

Strict Clippy gate:

```bash
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Result:

```text
Finished `dev` profile
```

Focused Phase 3 preflight:

```bash
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text
== Internal ROC Beta Phase 3 ron-policy reward-plan gate preflight passed ==
== policy remains declarative gate only; no receipt/balance/payout/finality truth, wallet/ledger mutation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`ron-policy` now proves for Phase 3 Round 1:

```text
policy can gate reviewed reward plans.
policy can require explicit wallet-boundary execution.
policy allow does not create receipt truth.
policy allow does not create balance truth.
policy allow does not create payout execution.
policy deny does not create refund truth.
policy deny does not create balance truth.
policy obligations cannot smuggle authority fields.
authority-shaped tags reject.
authority-shaped obligation params reject.
economics policy regressions remain green.
policy remains declarative only.
```

---

## 12. What this does not do yet

This Round 1 slice does **not** execute approved payouts.

Not yet complete:

```text
approved payout execution through svc-wallet
durable payout receipts in ron-ledger
duplicate payout prevention across the full execution path
payout receipt replay/conservation proof
policy-approved payout intent execution
Phase 3 overall completion
```

Those belong to Phase 3 Round 2.

---

## 13. Commands to rerun

From repo root:

```bash
cargo fmt -p ron-policy -- --check
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test economics_policy
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Phase 3 Round 1 completion context

Phase 3 Round 1 intended crate pairs are now green:

```text
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

Round 1 proved:

```text
accounting/reward-plan reference DTOs are non-authority.
ledger rejects reward-plan material as receipt truth.
accounting snapshots are deterministic derivative material.
wallet/accounting observer boundaries are preserved.
rewarder produces deterministic capped payout plans.
policy validates/gates reward plans declaratively.
no payout execution occurs yet.
```

---

## 15. Next phase context

Next active target:

```text
Internal ROC Beta Phase 3 Round 2 — approved payout execution through svc-wallet
```

Expected Round 2 work involving `ron-policy`:

```text
policy validates/gates payout plans.
policy rejects duplicate/uncapped payout attempts.
policy enforces cap/category/pool eligibility.
policy remains declarative.
policy never mutates wallet.
policy never mutates ledger.
policy never creates receipt/balance/finality truth.
```

Do not implement direct policy wallet/ledger mutation.

Do not let policy become payout execution authority.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

These are ready to append to each crate’s `NOTES.MD`.


### END NOTE - JUNE 29 2026 - 18:30 CST


### BEGIN NOTE - JUNE 29 2026 - 19:05 CST

Below are paste-ready notes for:

```text id="7zvcgt"
crates/svc-rewarder/NOTES.MD
crates/ron-policy/NOTES.MD
```

These cover **Internal ROC Beta Phase 3 Round 2 — `svc-rewarder + ron-policy`**, now **GREEN / PARKED**. Your terminal output confirms the new rewarder approved-payout intent test passed 5/5, the new policy approved-payout gate test passed 5/5, prior regressions passed, and both clippy gates passed. 

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

# svc-rewarder — Internal ROC Beta Phase 3 Round 2 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 2
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 Round 2 slice

---

## 0. Safe status label

```text id="6qvid6"
Internal ROC Beta Phase 3 Round 2 svc-rewarder approved-payout intent boundary is GREEN / PARKED.
```

Pair label:

```text id="6ysaz2"
Internal ROC Beta Phase 3 Round 2 svc-rewarder + ron-policy approved-payout intent/policy-gate boundary is GREEN / PARKED.
```

Phase label:

```text id="ca85xy"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

Current Phase 3 status:

```text id="1gfxuq"
Round 1: COMPLETE / GREEN / PARKED
Round 2: COMPLETE / GREEN / PARKED
Phase 3: COMPLETE / GREEN / PARKED
```

Current Phase 3 Round 2 crate-pair status:

```text id="2rlma8"
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

This does **not** mean the whole Internal ROC Beta is complete.

Next phase:

```text id="uir91a"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

---

## 1. What this slice added

This slice added focused Phase 3 Round 2 coverage proving that `svc-rewarder` emits deterministic, capped, wallet-issue handoff candidates only.

New test:

```text id="tal9vq"
crates/svc-rewarder/tests/internal_roc_beta_phase3_approved_payout_intent_boundary.rs
```

Updated script:

```text id="8ihuck"
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Follow-up fix:

```text id="uw4g59"
The approved payout cap/conservation test was corrected for floor rounding.
A requested cap of 333 can produce a deterministic payout handoff total of 332 with a 1-minor-unit residual.
The residual must stay out of wallet handoff truth.
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text id="atqtkv"
crates/svc-rewarder/tests/internal_roc_beta_phase3_approved_payout_intent_boundary.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 Round 2 test target contains five tests:

```text id="bs8x91"
approved_payout_intent_candidates_are_wallet_issue_handoff_only
category_pool_cap_and_conservation_survive_payout_intent_handoff
duplicate_payout_planning_markers_are_deterministic_but_not_execution_truth
wallet_issue_requests_remain_string_money_and_do_not_carry_receipt_truth
payout_handoff_dtos_reject_authority_smuggling_fields
```

These tests prove `svc-rewarder` remains deterministic payout planning and wallet handoff material only.

---

## 4. Boundary doctrine proven

`svc-rewarder` remains:

```text id="ecm9et"
deterministic capped payout planner
reward manifest producer
wallet issue handoff preview producer
idempotency/dedupe marker producer
audit/planning material producer
```

`svc-rewarder` does **not** become:

```text id="qdrzmh"
wallet authority
ledger authority
receipt truth
balance truth
approved payout executor
finality truth
bridge authority
staking authority
liquidity authority
external settlement authority
public-chain authority
```

Correct role:

```text id="lwl80t"
ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved payout mutation
→ ron-ledger durable receipt
```

`svc-rewarder` emits candidate handoff material. It does not execute payouts.

---

## 5. Approved payout intent handoff proven

The test confirms approved payout intent candidates are wallet issue handoff material only.

Proven handoff shape:

```text id="kuhejr"
SettlementBatch
WalletIssueBatch
WalletIssueRequest
```

Proven properties:

```text id="8sgtnd"
epoch_id is stable
run_key is stable
manifest_commitment is stable
funding_source is explicit
wallet_path points to wallet issue path
asset is roc
amount_minor is a decimal string
idempotency_key is present and bounded
memo labels rewarder planning source
recipient accounts are deterministically sorted
```

Important distinction:

```text id="kbw63e"
WalletIssueRequest is not a receipt.
WalletIssueBatch is not a balance.
SettlementBatch is not payout execution.
Rewarder handoff is not ledger mutation.
```

---

## 6. Cap, conservation, and floor residual behavior proven

The first version of this test expected exact equality between requested cap and payout handoff total.

The fix corrected the test to reflect actual integer floor rounding doctrine.

Correct behavior:

```text id="xlb3my"
requested_cap = 333
floor-rounded payout handoff total = 332
deterministic residual = 1
```

The corrected rule:

```text id="g9njd7"
settlement.total_minor_units <= requested_cap
wallet_batch.total_minor_units == settlement.total_minor_units
sum(wallet_issue_request.amount_minor) == settlement.total_minor_units
floor residual stays out of wallet issue requests
```

This is the right behavior.

The residual must not be silently pushed into a wallet issue request.

This proves:

```text id="sby0st"
category pool cap is enforced
wallet handoff total is conserved
floor rounding is deterministic
remainder is explicit by absence from payout handoff
no silent inflation occurs
```

---

## 7. Duplicate payout planning markers proven non-authoritative

The test confirms duplicate planning markers are deterministic dedupe markers, not execution truth.

Proven behavior:

```text id="6jqd44"
same sealed inputs produce identical settlement batches
same sealed inputs produce identical wallet issue batches
first handoff marker can be accepted
duplicate handoff marker returns dup
dry-run remains dry_run
```

Important rule:

```text id="4d3m1j"
rewarder idempotency/dedupe markers are not operation_id.
rewarder idempotency/dedupe markers are not ledger sequence.
rewarder idempotency/dedupe markers are not wallet receipt truth.
```

Actual duplicate payout prevention remains enforced downstream through `svc-wallet` and `ron-ledger`.

---

## 8. String-money handoff preserved

The test confirms wallet issue requests keep amount fields as string money.

Proven behavior:

```text id="0g4tdj"
amount_minor serializes as a string
amount_minor is nonzero after dust filtering
idempotency_key is present
wallet request does not carry receipt fields
wallet request does not carry balance fields
wallet request does not carry finality fields
wallet request does not carry bridge/staking/liquidity/exchange fields
```

This preserves the Internal ROC and QuickChain money doctrine:

```text id="tgs38h"
integer minor-unit strings only
no floats
no implicit client-side payout truth
```

---

## 9. Authority-smuggling rejected

The test confirms payout handoff DTOs reject authority-smuggling fields.

Rejected examples include:

```text id="xwvy7y"
receipt_id
receipt_hash
receipt_root
receipt_proof
accepted_receipt
wallet_receipt
ledger_receipt
balance
balance_minor
available_balance
wallet_balance
ledger_balance
balance_truth
receipt_truth
payout_execution_truth
wallet_mutation
ledger_mutation
operation_id
account_sequence
finality
finalized
checkpoint_hash
checkpoint_root
bridge_txid
solana_signature
rox_settlement_id
staking_position_id
staking_yield_bps
liquidity_pool_id
exchange_order_id
outside_settlement_claim
paid_unlock_authority
cache_unlock_authority
silent_spend
fake_balance
fake_receipt
```

This prevents the rewarder handoff layer from becoming a fake wallet/ledger authority surface.

---

## 10. Tests / gates passed

Focused Phase 3 Round 2 approved payout intent boundary:

```bash id="w3g4zo"
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
```

Result:

```text id="jmg881"
running 5 tests
test category_pool_cap_and_conservation_survive_payout_intent_handoff ... ok
test wallet_issue_requests_remain_string_money_and_do_not_carry_receipt_truth ... ok
test duplicate_payout_planning_markers_are_deterministic_but_not_execution_truth ... ok
test approved_payout_intent_candidates_are_wallet_issue_handoff_only ... ok
test payout_handoff_dtos_reject_authority_smuggling_fields ... ok

test result: ok. 5 passed; 0 failed
```

Focused Phase 3 Round 1 reward-plan boundary regression:

```bash id="xcdlnx"
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
```

Result:

```text id="i6c895"
running 5 tests
test raw_engagement_fields_cannot_be_rewarder_payout_input_authority ... ok
test reward_plan_enforces_pool_cap_and_conservation_without_execution ... ok
test reward_plan_is_deterministic_for_same_snapshot_regardless_input_order ... ok
test wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth ... ok
test reward_policy_and_compute_request_reject_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC rewarder planning non-authority regression:

```bash id="aguzvq"
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
```

Result:

```text id="bvtygq"
running 5 tests
test protocol_pool_planning_requires_signed_policy_and_stays_provenance_only ... ok
test reward_policy_rejects_paid_content_authority_poison_fields ... ok
test paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth ... ok
test wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt ... ok
test compute_request_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain no-direct-mutation regression:

```bash id="cuxc1l"
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
```

Result:

```text id="2nnqnn"
running 4 tests
test compute_request_still_rejects_direct_mutation_authority_smuggling ... ok
test planning_outputs_do_not_claim_receipts_balances_operation_truth_roots_or_finality ... ok
test config_rejects_external_settlement_bridge_anchor_validator_and_root_knobs ... ok
test router_does_not_expose_direct_wallet_ledger_quickchain_or_bridge_mutation_routes ... ok

test result: ok. 4 passed; 0 failed
```

QuickChain funding-source regression:

```bash id="t6hbam"
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
```

Result:

```text id="gxmrsk"
running 6 tests
test unsigned_protocol_pool_policy_is_rejected_by_validator ... ok
test policy_requires_explicit_funding_source_on_wire ... ok
test current_policy_accepts_explicit_protocol_pool_and_rejects_smuggled_authority_fields ... ok
test manifest_carries_funding_provenance_but_not_funding_finality ... ok
test wallet_preview_carries_batch_provenance_but_requests_remain_wallet_issue shape ... ok
test compute_request_rejects_top_level_funding_authority_smuggling ... ok

test result: ok. 6 passed; 0 failed
```

QuickChain replay/no-double-issue regression:

```bash id="hxxil3"
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
```

Result:

```text id="qw37ks"
running 4 tests
test duplicate_epoch_replay_is_dedupe_not_second_payout_authority ... ok
test same_snapshot_policy_and_epoch_produce_same_plan_commitment ... ok
test idempotency_keys_are_retry_dedupe_not operation identity ... ok
test reordered_snapshot_rows_produce_same_plan ... ok

test result: ok. 4 passed; 0 failed
```

Strict Clippy gate:

```bash id="3x1cxc"
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

Result:

```text id="ggexfl"
Finished `dev` profile
```

Focused preflight:

```bash id="noky2p"
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text id="tfipwb"
== Internal ROC Beta Phase 3 svc-rewarder approved-payout intent preflight passed ==
== rewarder emits deterministic capped wallet handoff candidates only; no wallet/ledger mutation, receipt/balance/finality truth, raw-engagement direct ROC allocation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`svc-rewarder` now proves for Phase 3 Round 2:

```text id="a5gpln"
approved payout intent candidates are deterministic.
approved payout intent candidates are capped.
approved payout handoff totals are conserved.
floor residuals are not silently issued.
wallet issue requests remain handoff DTOs only.
wallet issue requests use string money.
duplicate planning markers are dedupe markers, not execution truth.
rewarder does not mutate wallet.
rewarder does not mutate ledger.
rewarder does not create receipts.
rewarder does not create balances.
rewarder does not create finality.
rewarder does not enable bridge, staking, liquidity, exchange-facing, or external settlement behavior.
```

---

## 12. What this completes

This crate pair completed Phase 3 Round 2.

Together with earlier crate pairs, this completes Phase 3.

Safe overall label:

```text id="f9n2t4"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

---

## 13. Commands to rerun

From repo root:

```bash id="ihwyq6"
cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo clippy -p svc-rewarder --all-targets -- -D warnings
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash id="j9lc0x"
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Next phase context

Next phase:

```text id="j3ox13"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

Expected Phase 4 direction:

```text id="m9ffxa"
CrabLink displays backend-derived wallet receipts.
CrabLink displays backend-derived balances.
CrabLink keeps receipt cache display-only.
CrabLink performs explicit confirmation before spend.
CrabLink never invents receipt, balance, payout, finality, paid unlock, bridge, staking, liquidity, or external settlement truth.
```

Do not add new ledger mutation paths.

Do not let CrabLink, gateway, omnigate, index, storage, policy, accounting, or rewarder mutate balances.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

# ron-policy — Internal ROC Beta Phase 3 Round 2 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 2
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 Round 2 slice

---

## 0. Safe status label

```text id="5flb7n"
Internal ROC Beta Phase 3 Round 2 ron-policy approved-payout policy-gate boundary is GREEN / PARKED.
```

Pair label:

```text id="eeb8l2"
Internal ROC Beta Phase 3 Round 2 svc-rewarder + ron-policy approved-payout intent/policy-gate boundary is GREEN / PARKED.
```

Phase label:

```text id="x9rce7"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

Current Phase 3 status:

```text id="w6eagy"
Round 1: COMPLETE / GREEN / PARKED
Round 2: COMPLETE / GREEN / PARKED
Phase 3: COMPLETE / GREEN / PARKED
```

Current Phase 3 Round 2 crate-pair status:

```text id="w6m1h6"
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

This does **not** mean the whole Internal ROC Beta is complete.

Next phase:

```text id="wnuyjk"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

---

## 1. What this slice added

This slice added focused Phase 3 Round 2 coverage proving `ron-policy` gates approved payout candidates declaratively only.

New test:

```text id="syg6kh"
crates/ron-policy/tests/internal_roc_beta_phase3_approved_payout_policy_gate.rs
```

Updated script:

```text id="hlyvzi"
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text id="q6yhso"
crates/ron-policy/tests/internal_roc_beta_phase3_approved_payout_policy_gate.rs
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 Round 2 test target contains five tests:

```text id="xcx3sj"
approved_payout_policy_gate_allows_candidate_without_execution_truth
missing_duplicate_guard_marker_denies_without_refund_or_receipt_authority
approved_payout_policy_tags_reject_authority_shapes
approved_payout_policy_obligation_params_reject_authority_shapes
approved_payout_policy_gate_remains_declarative_not_execution_surface
```

These tests prove `ron-policy` can gate approved payout candidates, but cannot create payout execution, wallet mutation, ledger mutation, receipt truth, balance truth, finality truth, bridge, staking, liquidity, or external settlement truth.

---

## 4. Boundary doctrine proven

`ron-policy` remains:

```text id="2ho8ib"
declarative policy gate
allow/deny evaluator
obligation producer
economics config validator
safe review/checkpoint in the payout loop
```

`ron-policy` does **not** become:

```text id="85w351"
wallet authority
ledger authority
approved payout executor
receipt truth
balance truth
refund authority
finality truth
paid unlock authority
bridge authority
staking authority
liquidity authority
external settlement authority
```

Correct role:

```text id="7glb8k"
svc-rewarder payout intent candidate
→ ron-policy declarative validation/gating
→ svc-wallet approved payout mutation
→ ron-ledger durable receipt
```

Policy can say “allowed” or “denied.”

Policy cannot execute.

---

## 5. Approved payout gate allow behavior proven

The test confirms policy can allow an approved payout candidate only when safe declarative markers are present.

Safe markers include:

```text id="hirgok"
reward-plan-reviewed
bounded-pool-cap-checked
duplicate-payout-guard-checked
approved-payout-intent-candidate
svc-wallet-execution-required
policy-gate-only
```

Allowed decision remains declarative.

The safe obligation shape includes:

```text id="61vl9y"
require-approved-payout-intent-through-svc-wallet
plan_source = svc_rewarder
execution_boundary = svc_wallet
ledger_truth = ron_ledger
duplicate_guard = required
pool_cap = required
```

This is policy review material only.

It does not create a receipt, balance, finality, or payout execution.

---

## 6. Missing duplicate guard denial proven

The test confirms an approved payout candidate missing the duplicate guard marker denies.

Proven behavior:

```text id="ovb0rw"
missing duplicate-payout-guard-checked marker
→ policy denies
→ no refund obligation
→ no receipt obligation
→ no balance obligation
→ no wallet mutation obligation
→ no ledger mutation obligation
```

This matters because duplicate payout prevention is required for Phase 3 Round 2.

Policy denial is not an economic action.

---

## 7. Authority-shaped tags rejected

The test confirms approved payout policy tags reject authority-shaped material.

Rejected examples include:

```text id="mqf7ey"
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents policy tags from becoming proof, receipt, balance, finality, operation identity, or bridge authority.

---

## 8. Authority-shaped obligation params rejected

The test confirms obligation params reject authority-shaped material.

Rejected examples include:

```text id="eyv32l"
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents obligations from smuggling payout execution, wallet mutation, ledger mutation, proof, finality, receipt, balance, or external settlement authority.

---

## 9. Policy gate remains declarative

The test confirms safe policy obligations remain review/check material only.

Safe obligation examples:

```text id="9j8dkq"
require-approved-payout-intent-through-svc-wallet
record-policy-review-note
```

Safe params include:

```text id="4l3k85"
review = required
cap_check = required
duplicate_check = required
wallet_handoff = required
note = policy_review_only
```

Forbidden obligation meanings remain absent:

```text id="7guy1z"
issue
transfer
burn
capture
release
receipt
balance
finality
bridge
staking
liquidity
```

---

## 10. Tests / gates passed

Focused Phase 3 Round 2 approved payout policy gate:

```bash id="jjz8ds"
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
```

Result:

```text id="06e9rf"
running 5 tests
test missing_duplicate_guard_marker_denies_without_refund_or_receipt_authority ... ok
test approved_payout_policy_gate_allows_candidate_without_execution_truth ... ok
test approved_payout_policy_gate_remains_declarative_not_execution_surface ... ok
test approved_payout_policy_tags_reject_authority_shapes ... ok
test approved_payout_policy_obligation_params_reject_authority_shapes ... ok

test result: ok. 5 passed; 0 failed
```

Focused Phase 3 Round 1 reward-plan policy gate regression:

```bash id="y2yuec"
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
```

Result:

```text id="uuvbcu"
running 5 tests
test known_authority_shaped_reward_plan_policy_tags_reject ... ok
test known_authority_shaped_reward_plan_obligation_params_reject ... ok
test reward_plan_policy_denial_is_not_refund_receipt_or balance truth ... ok
test reward_plan_policy_gate_remains_declarative_not_execution_surface ... ok
test reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC paid-content policy non-authority regression:

```bash id="qz4454"
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
```

Result:

```text id="s5t89g"
running 5 tests
test policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority ... ok
test policy_rejects_paid_content_authority_shaped_tags ... ok
test policy_allow_after_backend_context_is_not_paid_unlock_or receipt truth ... ok
test economics_paid_content_view_prices_and_validates_capture_plan_without_authority ... ok
test economics_config_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain decision non-authority regression:

```bash id="na83dv"
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
```

Result:

```text id="54gqx1"
running 4 tests
test authority_shaped_obligation_param_key_rejects_policy ... ok
test authority_shaped_obligation_kind_rejects_policy ... ok
test deny_decision_is_still_not_receipt_balance_or finality truth ... ok
test allow_decision_is_policy_result_not paid unlock or receipt truth ... ok

test result: ok. 4 passed; 0 failed
```

Economics policy regression:

```bash id="vu53rg"
cargo test -p ron-policy --test economics_policy
```

Result:

```text id="28plnz"
running 15 tests
test float_value_rejects_during_parse ... ok
test capture_plan_accepts_required_dynamic_recipient ... ok
test capture_over_action_cap_rejects ... ok
test disabled_action_rejects_lookup_but_config_can_load ... ok
test invalid_split_sum_rejects ... ok
test deterministic_action_order_is_sorted ... ok
test missing_required_action_rejects ... ok
test missing_dynamic_recipient_rejects_capture_plan ... ok
test overflow_value_rejects_during_parse ... ok
test negative_value_rejects_during_parse ... ok
test paid_storage_put_price_uses_minimum_and_hold_multiplier ... ok
test unknown_action_rejects ... ok
test unknown_paid_action_lookup_rejects ... ok
test unknown_split_destination_rejects ... ok
test valid_checked_in_roc_economics_config_loads ... ok

test result: ok. 15 passed; 0 failed
```

Strict Clippy gate:

```bash id="z1lc05"
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Result:

```text id="x7xir2"
Finished `dev` profile
```

Focused preflight:

```bash id="foul30"
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text id="2b34gc"
== Internal ROC Beta Phase 3 ron-policy approved-payout gate preflight passed ==
== policy remains declarative gate only; no receipt/balance/payout/finality truth, wallet/ledger mutation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`ron-policy` now proves for Phase 3 Round 2:

```text id="r2s7vk"
approved payout candidates can be gated declaratively.
policy allow is not payout execution.
policy deny is not refund execution.
policy obligations are review/check material only.
missing duplicate guard markers deny safely.
authority-shaped tags reject.
authority-shaped obligation params reject.
policy does not mutate wallet.
policy does not mutate ledger.
policy does not create receipts.
policy does not create balances.
policy does not create finality.
policy does not unlock paid content.
policy does not enable bridge, staking, liquidity, exchange-facing, or external settlement behavior.
```

---

## 12. What this completes

This crate pair completed Phase 3 Round 2.

Together with earlier crate pairs, this completes Phase 3.

Safe overall label:

```text id="qp9bjs"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

---

## 13. Commands to rerun

From repo root:

```bash id="2n4yc5"
cargo fmt -p ron-policy -- --check
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test economics_policy
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash id="svdrdl"
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Next phase context

Next phase:

```text id="7408gy"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

Expected Phase 4 direction:

```text id="ycdq8i"
CrabLink displays backend-derived wallet receipts.
CrabLink displays backend-derived balances.
CrabLink keeps receipt cache display-only.
CrabLink performs explicit confirmation before spend.
CrabLink never invents receipt, balance, payout, finality, paid unlock, bridge, staking, liquidity, or external settlement truth.
```

Do not add new ledger mutation paths.

Do not let CrabLink, gateway, omnigate, index, storage, policy, accounting, or rewarder mutate balances.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

These are ready to append to each crate’s `NOTES.MD`.


### END NOTE - JUNE 29 2026 - 19:05 CST


### BEGIN NOTE - JUNE 29 2026 - 23:25 CST

Based on the latest bundles, `ron-policy` now has the Phase 5 economics TOML validator/preflight path, and `svc-rewarder` has the Phase 5 config-driven planning preflight path that checks the planning projection, Phase 5 test, Phase 3 regressions, and strict Clippy.   The active buildplan says this slice’s job was for `ron-policy` to validate TOML and bridge/staking inertness, while `svc-rewarder` consumes validated config for planning only with no hard-coded payout constants. 

# Crate Notes — Internal ROC Beta Phase 5 Round 1

## Crate Pair: `svc-rewarder + ron-policy`

## Status

`svc-rewarder + ron-policy` is now:

```text
Internal ROC Beta Phase 5 Round 1 svc-rewarder + ron-policy
Status: GREEN / PARKED
```

This completes the second Phase 5 Round 1 crate pair.

Completed Phase 5 Round 1 pairs so far:

```text
ron-proto + ron-ledger: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

Remaining Phase 5 Round 1 pair:

```text
svc-wallet + ron-accounting
```

---

## Purpose of this slice

This slice proved the Phase 5 tokenomics/config doctrine across the policy and reward-planning boundary:

```text
configs/roc-economics.toml is validated as canonical mutable economics config.
ron-policy validates tokenomics config only.
svc-rewarder consumes validated economics config for planning only.
Neither crate becomes receipt truth, balance truth, payout execution truth, finality truth, wallet authority, ledger authority, bridge runtime, staking runtime, liquidity, or external settlement.
```

The important architectural boundary remains:

```text
ron-policy = declarative validator/gate only
svc-rewarder = deterministic capped payout planner only
svc-wallet = only approved mutation front-door
ron-ledger = durable economic truth
```

---

## Files added or materially changed

## `ron-policy`

Added:

```text
crates/ron-policy/src/economics/internal_roc.rs
crates/ron-policy/tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs
crates/ron-policy/tests/fixtures/roc-paid-action-economics.legacy.toml
crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

Updated:

```text
crates/ron-policy/src/economics/mod.rs
crates/ron-policy/tests/economics_policy.rs
crates/ron-policy/tests/internal_roc_beta_paid_content_policy_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
```

## `svc-rewarder`

Added:

```text
crates/svc-rewarder/src/inputs/economics.rs
crates/svc-rewarder/tests/internal_roc_beta_phase5_config_driven_planning.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

Updated:

```text
crates/svc-rewarder/src/inputs/mod.rs
```

Shared/global config already created in the previous pair:

```text
configs/roc-economics.toml
```

---

## `ron-policy` notes

## What `ron-policy` now proves

`ron-policy` now has a dedicated canonical Internal ROC economics TOML validator.

It validates:

```text
schema == internal_roc.economics-config.v1
version == 1
money values are integer minor-unit strings
money values reject floats/numeric TOML numbers
bps totals equal 10000
remainder sink is explicit
configured_account sink requires a configured account
bridge placeholder remains disabled/inert
staking placeholder remains disabled/inert
unknown TOML fields are rejected
validation output does not claim authority truth
```

It explicitly does **not**:

```text
create receipts
create balances
create payouts
execute wallet mutations
mutate ledger
grant paid access
claim finality
activate bridge/staking/liquidity/external settlement
```

## Important implementation detail

The old `economics_policy.rs` regression expected the legacy paid-action economics TOML shape. The new canonical `configs/roc-economics.toml` uses the Phase 5 schema shape, so the legacy test was moved to a fixture:

```text
crates/ron-policy/tests/fixtures/roc-paid-action-economics.legacy.toml
```

This preserves the old paid-action pricing/split regression without forcing the new canonical Phase 5 config to pretend to be the legacy action-policy config.

Do not undo this split.

Correct model:

```text
configs/roc-economics.toml
  = canonical Internal ROC Phase 5 tokenomics config

tests/fixtures/roc-paid-action-economics.legacy.toml
  = legacy paid-action economics fixture for older economics_policy regressions
```

## `ron-policy` green proof

Latest passed targets:

```text
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
  7 passed / 0 failed

cargo test -p ron-policy --test economics_policy
  15 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
  5 passed / 0 failed

cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
  4 passed / 0 failed

cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
  passed
```

Phase 5 preflight passed:

```text
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh

== Internal ROC Beta Phase 5 ron-policy economics TOML validation preflight passed ==
== policy validates tokenomics config only; no receipt/balance/payout/finality truth, bridge, staking, liquidity, or external settlement ==
```

---

## `svc-rewarder` notes

## What `svc-rewarder` now proves

`svc-rewarder` now has an Internal ROC economics projection for reward planning:

```text
crates/svc-rewarder/src/inputs/economics.rs
```

It parses canonical Internal ROC economics TOML and projects only the planning-safe fields:

```text
schema
version
epoch_pool_cap_minor
max_reward_minor_per_account_per_epoch
max_reward_minor_per_content_per_epoch
rounding_mode
remainder_sink
bridge_inert
staking_inert
```

It validates:

```text
schema/version
positive integer minor-unit strings
reward_pools.category_caps is present/non-empty
anti_farming.max_events_per_account_per_epoch > 0
rounding.mode == floor
remainder_sink is explicit
configured_account sink requires account
non-configured sinks reject stray remainder_sink_account
future_bridge.enabled == false
future_bridge.state is explicit
future_staking.enabled == false
future_staking.state is explicit
```

It converts config-derived economics into a `RewardPolicy` only as planning input:

```text
InternalRocRewardPlanningEconomics::to_reward_policy(...)
```

That policy remains a planning cap source only. It does not become a wallet receipt, ledger effect, paid unlock, balance truth, or finality claim.

## Important fixes made

### 1. Test source scan path fixed

The Phase 5 source scan originally used:

```text
crates/svc-rewarder/src
```

But Cargo integration tests run with the crate root as the effective path context, so that path failed. It was corrected to use:

```text
Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
```

Keep this. Do not revert to a repo-root-relative path inside crate integration tests.

### 2. Dead-code warnings fixed by actually validating fields

These fields were parsed only for strict TOML shape validation and initially triggered warnings:

```text
reward_pools.category_caps
anti_farming.max_events_per_account_per_epoch
rounding.remainder_sink_account
future_bridge.state
future_staking.state
```

They are now read and validated. Keep that validation because strict Clippy is part of the preflight.

### 3. Bool assert Clippy fixed

The test originally had:

```rust
assert_eq!(manifest.ledger.emitted, false);
```

It was fixed to:

```rust
assert!(!manifest.ledger.emitted);
```

Keep this to avoid `clippy::bool_assert_comparison`.

## `svc-rewarder` green proof

Latest passed targets:

```text
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
  3 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
  5 passed / 0 failed

cargo clippy -p svc-rewarder --all-targets -- -D warnings
  passed
```

Phase 5 preflight passed:

```text
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh

== Internal ROC Beta Phase 5 svc-rewarder config-driven planning preflight passed ==
== rewarder consumes validated economics config for planning only; no hard-coded payout constants or wallet/ledger authority ==
```

---

## Final terminal proof for this crate pair

The last terminal output showed:

```text
svc-rewarder Phase 5 config-driven planning: 3 passed / 0 failed
svc-rewarder Phase 3 reward-plan boundary: 5 passed / 0 failed
svc-rewarder Phase 3 approved-payout intent boundary: 5 passed / 0 failed
svc-rewarder strict clippy: passed
svc-rewarder Phase 5 preflight: passed
```

Previous output showed:

```text
ron-policy Phase 5 economics TOML validation: 7 passed / 0 failed
ron-policy economics_policy regression: 15 passed / 0 failed
ron-policy strict clippy: passed
ron-policy Phase 5 preflight: passed
```

Therefore:

```text
Internal ROC Beta Phase 5 Round 1 svc-rewarder + ron-policy economics TOML validation/config-driven planning slice is GREEN / PARKED.
```

---

## Architecture boundaries preserved

This slice preserved the Internal ROC authority model:

```text
ron-policy validates/gates only.
ron-policy output is never receipt truth.
ron-policy output is never balance truth.
ron-policy output is never payout execution truth.
ron-policy output is never finality truth.

svc-rewarder plans payouts only.
svc-rewarder consumes accounting/config/policy inputs only.
svc-rewarder emits deterministic wallet handoff candidates only.
svc-rewarder does not mutate ledger.
svc-rewarder does not bypass svc-wallet.
svc-rewarder does not invent balances.
svc-rewarder does not invent receipts.
svc-rewarder does not treat config as authority.
svc-rewarder does not treat raw engagement as direct ROC allocation.
```

The only allowed future economic mutation path remains:

```text
approved payout plan
  -> policy gate
  -> explicit approved wallet execution
  -> svc-wallet
  -> ron-ledger
  -> backend receipt/balance truth
```

---

## Regression commands to keep

## `ron-policy`

```bash
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

## `svc-rewarder`

```bash
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
cargo clippy -p svc-rewarder --all-targets -- -D warnings
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

## Pair-level low-disk proof

```bash
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

---

## Do not regress

Do not let `configs/roc-economics.toml` become:

```text
receipt authority
balance authority
wallet mutation authority
ledger mutation authority
paid unlock authority
finality authority
bridge runtime authority
staking runtime authority
liquidity/exchange-facing authority
```

Do not let `ron-policy`:

```text
issue ROC
transfer ROC
burn ROC
open/capture/release holds
create receipts
create balances
grant unlocks
execute payouts
create finality
```

Do not let `svc-rewarder`:

```text
hard-code Phase 5 payout constants in business logic
consume raw engagement as direct payout authority
call ron-ledger directly for mutation
create wallet receipts
invent balances
silently execute payouts
treat dry-run manifests as settlement truth
activate bridge/staking/liquidity/external settlement
```

---

## Next crate pair

Next Phase 5 Round 1 crate pair:

```text
svc-wallet + ron-accounting
```

Expected purpose:

```text
svc-wallet:
  - prove economics config cannot directly mutate wallet/ledger
  - prove config cannot create receipt/balance truth
  - keep all mutation behind explicit wallet operation paths

ron-accounting:
  - label config version/hash/source in snapshots/reports only
  - prove accounting labels are derivative metadata, not balance truth
  - prove accounting cannot mutate ledger
```

Expected next-slice doctrine:

```text
config validates economics parameters
accounting can report config version/hash/source labels
wallet may use approved inputs only through explicit operation paths
neither config nor accounting can directly issue/transfer/burn/hold/capture/release ROC
ron-ledger remains the durable truth
```

Suggested next status label before work begins:

```text
Internal ROC Beta Phase 5 Round 1 svc-wallet + ron-accounting: NOT STARTED
```


### END NOTE - JUNE 29 2026 - 23:25 CST




### BEGIN NOTE - JUNE 29 2026 - 23:55 CST

The buildplan’s Round 2 gate is exactly this: prove raw engagement cannot directly mint/allocate ROC, isolate `analytics_only` and `metering`, require verification/caps/policy for `proof_eligible`, require explicit budget for `ad_budgeted`, and keep reward plans non-mutating.  Your terminal output shows the new `ron-accounting` 6/6 event-class test passed and the `svc-rewarder` 5/5 anti-farming test passed, with the only blocker being a Clippy unused import that you repaired. 

# Light Crate Notes — Internal ROC Beta Phase 5 Round 2

## Crate Pair: `ron-accounting + svc-rewarder`

## Status

```text
Internal ROC Beta Phase 5 Round 2 ron-accounting + svc-rewarder
Status: GREEN / PARKED
```

This slice proved the first Phase 5 Round 2 anti-farming/event-class boundary:

```text
Raw engagement cannot directly mint or allocate protocol ROC.
analytics_only stays quarantined.
metering does not directly become payout material.
proof_eligible requires verification/caps/policy before reward planning.
ad_budgeted requires explicit non-protocol budget.
svc-rewarder consumes only eligible/capped inputs.
Reward plans remain non-mutating.
```

---

## `ron-accounting` notes

Added:

```text
crates/ron-accounting/src/accounting/event_class.rs
crates/ron-accounting/tests/internal_roc_beta_phase5_event_class_antifarming.rs
crates/ron-accounting/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

Updated exports:

```text
crates/ron-accounting/src/accounting/mod.rs
crates/ron-accounting/src/lib.rs
```

What it now proves:

```text
economic_receipt requires backend wallet/ledger source.
analytics_only quarantines raw engagement.
metering never directly becomes payout or receipt truth.
proof_eligible service metrics require verification before planning.
ad_budgeted events require explicit budget and do not mint protocol ROC.
event-class decisions reject authority poisoning and unknown fields.
```

Green proof:

```text
cargo test -p ron-accounting --test internal_roc_beta_phase5_event_class_antifarming
  6 passed / 0 failed

cargo test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary
  5 passed / 0 failed

cargo test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority
  4 passed / 0 failed

bash crates/ron-accounting/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
  passed
```

Do not regress:

```text
ron-accounting must not create balance truth.
ron-accounting must not create receipt truth.
ron-accounting must not mutate wallet or ledger.
ron-accounting must not let raw client events claim economic_receipt.
ron-accounting must not let analytics_only or metering become direct payout material.
```

---

## `svc-rewarder` notes

Added:

```text
crates/svc-rewarder/src/inputs/anti_farming.rs
crates/svc-rewarder/tests/internal_roc_beta_phase5_antifarming_event_gates.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

Updated exports:

```text
crates/svc-rewarder/src/inputs/mod.rs
```

What it now proves:

```text
Verified proof_eligible inputs are capped before planning.
analytics_only is rejected as reward-planning input.
metering is rejected as direct reward-planning input.
unverified proof_eligible candidates are rejected.
ad_budgeted material requires explicit budget.
ad_budgeted material cannot use protocol-pool emission.
capped inputs produce deterministic dry-run manifests.
anti-farming gates have no wallet/ledger authority shortcuts.
```

Green proof:

```text
cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
  3 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
  5 passed / 0 failed

bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
  passed after unused-import repair
```

Small fix made:

```text
Removed unused AccountContribution import from:
crates/svc-rewarder/tests/internal_roc_beta_phase5_antifarming_event_gates.rs
```

Do not regress:

```text
svc-rewarder must not mutate ledger.
svc-rewarder must not create wallet receipts.
svc-rewarder must not treat raw engagement as payout authority.
svc-rewarder must not let ad_budgeted use protocol-pool emission.
svc-rewarder must not bypass policy or wallet.
svc-rewarder must not introduce bridge/staking/liquidity/external settlement.
```

---

## Pair-level boundary preserved

```text
ron-accounting classifies and snapshots.
svc-rewarder gates, caps, and plans.
ron-policy still gates eligibility next.
svc-wallet remains the only approved payout execution front-door.
ron-ledger remains durable balance/receipt truth.
```

Correct flow remains:

```text
classified event
→ ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved mutation
→ ron-ledger durable receipt
```

---

## Keep these regression commands

```bash
cargo test -p ron-accounting --test internal_roc_beta_phase5_event_class_antifarming
cargo test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary
cargo test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority
bash crates/ron-accounting/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh

cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

---

## Next crate pair

```text
ron-policy + svc-rewarder
```

Next purpose:

```text
Policy gates reward-plan eligibility and rejects uncapped raw engagement, analytics_only reward material, and metering direct payout.
```

Phase 5 Round 2 is now partly green: `ron-accounting + svc-rewarder` is parked, and `ron-policy + svc-rewarder` is next.


### END NOTE - JUNE 29 2026 - 23:55 CST


### BEGIN NOTE - JUNE 30 2026 - 00:30 CST

Here are the crate notes for the now-green Phase 5 Round 2 `svc-rewarder + ron-policy` slice. The newer bundles show the new Round 2 preflight scripts and tests are present in both crates.  

# Crate Notes — Internal ROC Beta Phase 5 Round 2

## Crate Pair: `svc-rewarder + ron-policy`

## Status

```text id="b4ld8d"
Internal ROC Beta Phase 5 Round 2 svc-rewarder + ron-policy
Status: GREEN / PARKED
```

This crate pair completed the policy-gated anti-farming slice.

---

## Pair-level result

This round proved:

```text id="5s2tyq"
ron-policy gates reward eligibility declaratively.
svc-rewarder consumes verified/capped/policy-gated inputs only.
raw engagement cannot become protocol ROC payout material.
analytics_only cannot enter reward planning.
metering cannot directly enter reward planning.
proof_eligible requires verification, caps, and policy gate.
ad_budgeted requires explicit non-protocol budget.
rewarder remains planning-only.
policy remains declarative-only.
```

Still preserved:

```text id="rqhm9f"
No wallet mutation from policy.
No wallet mutation from rewarder anti-farming gates.
No ledger mutation from policy.
No ledger mutation from rewarder planning.
No fake receipt truth.
No fake balance truth.
No fake finality.
No bridge runtime.
No staking runtime.
No liquidity.
No external settlement.
```

---

# `ron-policy` notes

## Added / updated

Added:

```text id="w2c4ag"
crates/ron-policy/tests/internal_roc_beta_phase5_antifarming_policy_gate.rs
crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

Hardened:

```text id="ys8kqz"
crates/ron-policy/src/parse/validate.rs
```

The validator was hardened so authority-shaped staking/liquidity terms cannot sneak into policy tags or obligations.

Important fixed cases:

```text id="jpw37m"
staking_position_id now rejects as authority-shaped tag material.
staking-position now rejects as authority-shaped obligation material.
```

## What `ron-policy` now proves

```text id="b7kk0d"
Verified/capped proof_eligible material may pass only as a declarative policy gate.
Raw engagement, analytics_only, and direct metering reward attempts deny.
Unverified proof_eligible material denies.
Uncapped proof_eligible material denies.
ad_budgeted material requires explicit budget.
ad_budgeted material requires non-protocol budget.
Authority-shaped tags reject during parsing.
Authority-shaped obligations reject during parsing.
Policy decisions do not claim receipt, balance, finality, wallet, or ledger truth.
```

## Green proof

```text id="c1nxmd"
cargo test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate
  7 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
  7 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
  5 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
  5 passed / 0 failed

cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
  4 passed / 0 failed

cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
  passed

bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
  passed
```

## Do not regress

```text id="r8zppm"
ron-policy must remain declarative.
ron-policy must not create payout execution truth.
ron-policy must not create receipt truth.
ron-policy must not create balance truth.
ron-policy must not unlock paid content.
ron-policy must not mutate wallet or ledger.
ron-policy must reject authority-shaped policy tags/obligations.
ron-policy must keep bridge/staking/liquidity/external settlement forbidden.
```

---

# `svc-rewarder` notes

## Added / updated

Added:

```text id="lb2yk5"
crates/svc-rewarder/tests/internal_roc_beta_phase5_policy_gate_interlock.rs
```

Updated:

```text id="p12o9h"
crates/svc-rewarder/src/inputs/anti_farming.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

Key model change:

```text id="e09i4l"
CappedRewardInputCandidate now requires policy_gate_passed.
```

This makes the rewarder gate stricter:

```text id="e7aide"
verification + caps alone are not enough.
proof_eligible must also pass ron-policy.
ad_budgeted must pass ron-policy and use explicit non-protocol budget.
```

## What `svc-rewarder` now proves

```text id="qplpcy"
Verified/capped candidate without policy gate is rejected.
Verified/capped/policy-gated proof_eligible candidate may enter planning.
ad_budgeted candidate requires policy gate.
ad_budgeted candidate requires explicit budget.
ad_budgeted candidate cannot use protocol-pool emission.
Policy-gated capped inputs produce deterministic dry-run manifests only.
Rewarder remains non-mutating.
Rewarder source has no wallet/ledger authority shortcuts.
```

## Green proof

```text id="qtp7y7"
cargo test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
  3 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
  5 passed / 0 failed

cargo clippy -p svc-rewarder --all-targets -- -D warnings
  passed

bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
  passed
```

## Do not regress

```text id="xk922d"
svc-rewarder must not accept raw engagement as payout input.
svc-rewarder must not accept analytics_only as payout input.
svc-rewarder must not accept metering as direct payout input.
svc-rewarder must not accept proof_eligible without verification/caps/policy gate.
svc-rewarder must not accept ad_budgeted without explicit non-protocol budget.
svc-rewarder must not mutate ledger.
svc-rewarder must not create wallet receipt truth.
svc-rewarder must not bypass ron-policy.
svc-rewarder must not bypass svc-wallet.
svc-rewarder must not introduce bridge/staking/liquidity/external settlement.
```

---

# Round 2 pair boundary

Correct flow is now reinforced:

```text id="a91cga"
classified event
→ ron-accounting snapshot/report
→ svc-rewarder anti-farming cap gate
→ ron-policy declarative eligibility gate
→ svc-rewarder deterministic payout plan
→ svc-wallet approved payout execution only
→ ron-ledger durable receipt/balance truth
```

This is forbidden:

```text id="gne8bq"
raw engagement
→ rewarder payout

analytics_only
→ rewarder payout

metering
→ direct payout

policy allow
→ wallet mutation

rewarder plan
→ ledger mutation

ad_budgeted
→ protocol-pool emission
```

---

# Current Phase 5 Round 2 status

```text id="i4okqy"
ron-accounting + svc-rewarder: GREEN / PARKED
ron-policy + svc-rewarder: GREEN / PARKED
```

Phase 5 Round 2 has now locked the main anti-farming spine:

```text id="m4yqjw"
event classification
→ anti-farming caps
→ policy gate
→ deterministic non-mutating reward plan
```

---

# Keep these commands

```bash id="k0rua7"
cargo test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh

cargo test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

---

# Next likely step

```text id="htl734"
Phase 5 closeout / reproducible smoke consolidation
```

Optional before closeout:

```text id="rul8av"
Add a focused svc-ads ad_budgeted slice only if svc-ads is active enough and we intentionally want ad-budgeted proof before Phase 5 closeout.
```

These notes are safe to carry forward: the main `svc-rewarder + ron-policy` Round 2 authority boundary is green and parked.


### END NOTE - JUNE 30 2026 - 00:30 CST


### BEGIN NOTE - JULY 1 2026 - 20:30 CST

## Pair 3 — `svc-rewarder + ron-policy`

Safe label:

```text id="8pw8m2"
Internal ROC Stabilization / Product Beta Readiness —
svc-rewarder + ron-policy capped reward planning / declarative policy gate boundary:
COMPLETE / GREEN / PARKED.
```

## What we changed

We added a stabilization wrapper around the reward-planning and policy-gating layer of the Internal ROC value loop.

For `svc-rewarder`, we added/locked:

```text id="1rg56i"
crates/svc-rewarder/docs/internal-roc-stabilization-reward-policy-gate.md
crates/svc-rewarder/tests/internal_roc_stabilization_reward_policy_gate_boundary.rs
crates/svc-rewarder/scripts/dev-internal-roc-stabilization-reward-policy-gate-preflight.sh
```

For `ron-policy`, we added/locked:

```text id="qkg4d4"
crates/ron-policy/docs/internal-roc-stabilization-policy-gate-non-authority.md
crates/ron-policy/tests/internal_roc_stabilization_policy_gate_non_authority_boundary.rs
crates/ron-policy/scripts/dev-internal-roc-stabilization-policy-gate-preflight.sh
```

And the aggregate pair gate:

```text id="jmwbx4"
scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh
```

We also repaired the new `ron-policy` stabilization wrapper so it checked semantic policy/economics boundaries instead of brittle exact spelling markers like `first-match`, `analytics_only`, or `validate_economics_policy`.

## What we accomplished

`svc-rewarder` was locked as **deterministic capped reward planning and wallet-handoff only**.

That means:

```text id="iz1qm4"
- rewarder consumes accounting snapshots and policy-gated candidate material
- rewarder applies anti-farming caps
- rewarder rejects raw engagement as payout authority
- rewarder rejects analytics_only and metering as direct payout material
- rewarder requires proof_eligible material to be verified, capped, and policy-gated
- rewarder requires ad_budgeted material to have explicit non-protocol budget authorization
- rewarder creates deterministic manifests and wallet issue request DTOs
- rewarder does not create receipts
- rewarder does not create balance truth
- rewarder does not mutate the ledger directly
- rewarder does not execute payouts except by handing approved requests to svc-wallet
```

`ron-policy` was locked as **declarative gate and economics validation only**.

That means:

```text id="9h1htj"
- policy can allow, deny, explain, and require obligations
- policy can validate ROC economics TOML
- policy can reject unsafe economics config
- policy can gate reward eligibility
- policy can gate approved payout candidates
- policy can reject raw engagement, analytics-only, direct metering, uncapped, unverified, or unfunded reward material
- policy decisions are not receipts
- policy decisions are not balances
- policy obligations are not payout execution
- economics config is not wallet authority
- policy never mutates wallet or ledger state
```

## Tests/gates that were parked

For `svc-rewarder`, the gate covered:

```text id="tm13zx"
- internal_roc_stabilization_reward_policy_gate_boundary
- internal_roc_beta_rewarder_planning_non_authority
- internal_roc_beta_phase3_reward_plan_boundary
- internal_roc_beta_phase3_approved_payout_intent_boundary
- internal_roc_beta_phase5_config_driven_planning
- internal_roc_beta_phase5_antifarming_event_gates
- internal_roc_beta_phase5_policy_gate_interlock
- quickchain_preflight_no_direct_mutation
- quickchain_preflight_replay_no_double_issue
- clippy clean
```

For `ron-policy`, the gate covered:

```text id="d72fco"
- internal_roc_stabilization_policy_gate_non_authority_boundary
- internal_roc_beta_paid_content_policy_non_authority
- internal_roc_beta_phase3_reward_plan_policy_gate
- internal_roc_beta_phase3_approved_payout_policy_gate
- internal_roc_beta_phase5_economics_toml_policy_validation
- internal_roc_beta_phase5_antifarming_policy_gate
- quickchain_preflight_decision_non_authority
- quickchain_preflight_economics_config_non_authority
- economics_policy
- clippy clean
```

## Boundary we proved

This pair locked the separation between **planning** and **gating**:

```text id="exqu9c"
svc-rewarder plans capped deterministic rewards.
ron-policy gates eligibility declaratively.
svc-wallet executes approved payout mutations.
ron-ledger records durable truth.
```

Correct path:

```text id="f673c3"
ron-accounting snapshot / verified evidence
→ svc-rewarder candidate classification
→ anti-farming caps
→ ron-policy declarative gate
→ deterministic reward manifest
→ wallet issue request DTO
→ svc-wallet approved payout execution
→ ron-ledger durable receipt/balance truth
```

Forbidden path:

```text id="i70gk1"
raw engagement
→ direct ROC payout

analytics_only
→ reward material

metering
→ direct payout

policy allow
→ receipt truth

policy allow
→ paid unlock

rewarder manifest
→ balance truth

rewarder wallet issue request
→ receipt truth before svc-wallet/ron-ledger acceptance
```

## Why this pair mattered

This pair was the **anti-farming and reward safety lock**.

Earlier pairs proved that ledger truth and wallet mutation are protected. This pair proved that the upstream reward engine cannot be tricked into converting low-quality or fakeable signals into ROC issuance.

The key doctrine locked here:

```text id="z5ax6q"
A reward plan is not a payout.
A policy allow is not a receipt.
A wallet issue request is not ledger truth.
Raw engagement is not mint authority.
Analytics are not payout authority.
Caps and policy gates are mandatory before reward material becomes payout-eligible.
```

## Final record note

```text id="5e7c7q"
Pair 3 completed the capped reward-planning and declarative policy-gate stabilization layer. svc-rewarder is now parked as deterministic, capped, anti-farming-aware reward planning and wallet-handoff infrastructure only. ron-policy is now parked as declarative allow/deny/obligation and ROC economics validation infrastructure only. The pair blocks raw-engagement payout, analytics-only payout, direct-metering payout, unverified or uncapped proof-eligible payout material, unfunded ad-budgeted protocol-pool emission, policy-created receipt/balance/finality truth, rewarder-created payout execution truth, direct ledger mutation, and all bridge/ROX/Solana/staking/liquidity/external-settlement drift.
```

Next pair in the record:

```text id="3q77tc"
svc-gateway + omnigate
```


### END NOTE - JULY 1 2026 - 20:30 CST




### BEGIN NOTE - JULY 13 2026 - 14:10 CST

Below is a paste-ready changelog for the `svc-rewarder` work completed during this session. The current codebundle confirms the new economics, Service Node planning, accounting handoff, and test surfaces. 

# `svc-rewarder` Changelog — BUILD_PLAN_Z Phase 14

**Session date:** July 13, 2026
**Repository:** `/Users/mymac/Desktop/RustyOnions`
**Crate:** `crates/svc-rewarder`
**Primary phase:** BUILD_PLAN_Z Phase 14 — Accounting Snapshots and Reward Plan Determinism
**Session posture:** Real Rust implementation, focused tests, strict validation, deterministic planning, and preserved non-authority boundaries.

---

## 1. Executive summary

This session substantially completed the `svc-rewarder` side of BUILD_PLAN_Z Phase 14.

The crate now has a dedicated deterministic planning path that:

```text
ron-accounting canonical epoch snapshot
    ->
validated accounting-to-rewarder handoff
    ->
recipient-free Service Node candidates
    ->
economics-bound Service Node reward plan
```

The implementation now preserves and validates:

```text
service_node_id
evidence class
content identity
accounting snapshot CID
economics configuration hash
policy hash
event counts
eligible work scores
category caps
per-node caps
per-content caps
epoch caps
challenge posture
policy-gate posture
```

User Node verification evidence is also transformed into deterministic neutral planning points without assigning a hardcoded ROC rate or payout recipient.

The rewarder remains a planning service. It does not become wallet, ledger, receipt, balance, consensus, or finality authority.

---

# 2. Files added

## 2.1 `src/core/service_node_plan.rs`

Added a new pure deterministic Service Node reward-planning module.

This module owns the Phase 14 Service Node planning model and includes:

```text
ServiceNodeRewardEvidenceClass
ServiceNodeRewardCandidate
ServiceNodeRewardPlanInput
ServiceNodeRewardAllocation
ServiceNodeRewardPlanTotals
ServiceNodeRewardPlan
compute_service_node_reward_plan
```

It also defines stable schema/version constants:

```text
SERVICE_NODE_REWARD_PLAN_SCHEMA
SERVICE_NODE_REWARD_PLAN_VERSION
```

The plan is explicitly identity-bound, deterministic, capped, and non-authoritative.

---

## 2.2 `src/inputs/accounting_epoch.rs`

Added the canonical `ron-accounting` epoch-snapshot adapter.

This module converts a validated:

```text
ron_accounting::AccountingEpochSnapshotV1
```

into:

```text
ServiceNodeRewardPlanInput
UserVerificationPointPlanV1
AccountingEpochRewardPlanningHandoffV1
```

The adapter verifies the accounting artifact before reward planning and prevents the rewarder from accepting free-form identities, categories, recipient accounts, amounts, or economics bindings.

New primary types include:

```text
UserVerificationPlannedPointV1
UserVerificationPointPlanV1
AccountingEpochRewardPlanningHandoffV1
```

New stable schema/version constants include:

```text
ACCOUNTING_EPOCH_REWARD_HANDOFF_SCHEMA
ACCOUNTING_EPOCH_REWARD_HANDOFF_VERSION
USER_VERIFICATION_POINT_PLAN_SCHEMA
USER_VERIFICATION_POINT_PLAN_VERSION
```

---

## 2.3 `tests/internal_roc_beta_phase14d_service_node_reward_plan.rs`

Added eight focused tests for identity-bound Service Node planning.

The tests prove:

```text
candidate order does not change the plan
category caps change actual allocations
per-node caps apply across content rows
per-content caps apply across multiple nodes
event-count caps reject farming
policy and challenge states fail closed
arbitrary recipients are rejected
caller-selected reward amounts are rejected
economics changes alter plan identity
```

---

## 2.4 `tests/internal_roc_beta_phase14d_accounting_epoch_handoff.rs`

Added six focused tests for the canonical accounting-to-rewarder handoff.

The tests prove:

```text
canonical accounting snapshots produce Service Node plan inputs
accepted User Node verification rows produce neutral points
accounting snapshot row order does not alter output
accounting economics mismatches fail closed
policy-only evidence cannot enter reward planning
User Node event-count caps are enforced
arbitrary recipients and reward amounts are rejected
```

---

# 3. Files modified

## 3.1 `Cargo.toml`

Promoted `ron-accounting` from a test-only development dependency to a normal crate dependency:

```toml
ron-accounting = { path = "../ron-accounting" }
```

### Reason

The canonical Phase 14 runtime planning adapter now consumes real `ron-accounting` epoch-snapshot and evidence-row types.

This avoids duplicating accounting DTOs or creating a second incompatible snapshot model inside `svc-rewarder`.

---

## 3.2 `src/core/mod.rs`

Registered and publicly exported the new Service Node planning module.

Added:

```rust
pub mod service_node_plan;
```

Exported:

```text
compute_service_node_reward_plan
ServiceNodeRewardAllocation
ServiceNodeRewardCandidate
ServiceNodeRewardEvidenceClass
ServiceNodeRewardPlan
ServiceNodeRewardPlanInput
ServiceNodeRewardPlanTotals
SERVICE_NODE_REWARD_PLAN_SCHEMA
SERVICE_NODE_REWARD_PLAN_VERSION
```

### Reason

The new planning surface must be accessible to the accounting adapter, tests, and future registry-resolution path without coupling callers to internal file layout.

---

## 3.3 `src/inputs/mod.rs`

Registered the new accounting epoch adapter:

```rust
pub mod accounting_epoch;
```

Exported its handoff and User Node point-plan types and functions.

Also exported the new category-planning type and economics-aware anti-farming helper added during this session.

### Reason

This keeps `inputs` as the stable crate facade for:

```text
accounting snapshots
economics projections
anti-farming gates
policy inputs
Phase 14 canonical handoffs
```

---

## 3.4 `src/inputs/economics.rs`

Extended the `ron-policy` economics projection with reward-category configuration.

Added:

```text
InternalRocRewardCategoryPlanningCap
category_caps
category_cap(...)
effective_category_pool_cap(...)
```

Each projected category now contains:

```text
category
pool_bps
category_cap_minor
```

Category rows are projected from the validated `ron-policy` economics model and sorted by stable category label.

### New category validation

The rewarder now verifies that:

```text
the category list is not empty
category labels are canonical
categories are sorted
categories are unique
pool_bps is nonzero
category caps are nonzero
category caps do not exceed the epoch cap
category basis points total exactly 10,000
```

### Effective category-pool calculation

The effective category ceiling now applies:

```text
available pool
    capped by epoch_pool_cap_minor
    multiplied by category pool_bps
    divided using deterministic floor math
    capped by category_cap_minor
```

Conceptually:

```text
effective category cap =
min(
    floor(
        min(available_pool, epoch_cap)
        × category_bps
        ÷ 10,000
    ),
    absolute_category_cap
)
```

All arithmetic remains integer-only and checked.

### Reason

Previously, economics identity included category configuration, but the rewarder did not project or calculate real category ceilings.

The new implementation makes category configuration usable by real reward-plan behavior while keeping the schema and validation owned by `ron-policy`.

---

## 3.5 `src/inputs/anti_farming.rs`

Added economics-derived per-account event-count enforcement.

New public helper:

```text
capped_contributions_from_candidates_with_economics
```

The helper:

```text
validates the selected economics projection
reads max_events_per_account_per_epoch
counts candidates by canonical account
rejects accounts above the configured event limit
aggregates multiple accepted events for the same account
applies existing counter caps
applies the existing score cap
returns deterministic account ordering
```

### Compatibility behavior

The original:

```text
capped_contributions_from_candidates
```

remains available.

It delegates to the shared implementation with an effectively unlimited separate event-count ceiling, preserving compatibility for existing callers.

### Same-account aggregation

Multiple accepted candidates for one account are now combined using checked arithmetic.

Aggregated counters remain bounded by:

```text
max_bytes_stored
max_bytes_served
max_uptime_seconds
max_score_per_account
```

### Reason

The economics profile already defined:

```text
max_events_per_account_per_epoch
```

but the real candidate-processing path did not consume it.

This session connected that economics-owned anti-farming limit to actual reward input processing.

---

## 3.6 `tests/internal_roc_beta_phase14d_economics_manifest_binding.rs`

Expanded the Phase 14D economics test target from seven tests to eleven tests.

Added coverage for:

```text
canonical category-cap ordering
node_delivery category lookup
basis-point category ceiling
absolute category ceiling
unknown-category rejection
category-cap changes altering economics identity
category-cap changes altering effective allocation ceilings
```

The test suite now proves that category configuration is not inert metadata.

---

## 3.7 `tests/internal_roc_beta_phase5_antifarming_event_gates.rs`

Expanded the anti-farming target from five tests to seven tests.

Added coverage for:

```text
same-account events aggregate deterministically
configured event-count overages fail closed
```

The tests use a complete modified economics profile rather than injecting an isolated hardcoded runtime limit.

---

# 4. Service Node reward-plan implementation

## 4.1 Trusted planning input

The new `ServiceNodeRewardCandidate` contains:

```text
service_node_id
evidence_class
content_id
evidence_count
eligible_score
evidence_verified
accounting_accepted
policy_gate_passed
challenge_required
challenge_accepted
```

It deliberately does not contain:

```text
payout_recipient
wallet_account
requested_reward
requested_amount
reward_rate
wallet authority
ledger authority
```

Unknown fields are rejected through:

```rust
#[serde(deny_unknown_fields)]
```

This blocks recipient and amount smuggling at the serialized input boundary.

---

## 4.2 Evidence classes

The Service Node planning enum supports:

```text
delivery
availability
range_request
repair
hot_cache
```

All currently map to the economics-owned:

```text
node_delivery
```

category.

Policy-refusal and moderation-only evidence are not part of the rewardable Service Node enum.

They remain policy, challenge, review, or moderation material rather than direct reward candidates.

---

## 4.3 Candidate validation

Each Service Node candidate must pass:

```text
canonical service_node_id validation
canonical b3 content ID validation
known economics category lookup
evidence_count > 0
eligible_score > 0
evidence_verified = true
accounting_accepted = true
policy_gate_passed = true
resolved challenge posture
```

Challenge rules are fail-closed:

```text
challenge required + not accepted -> reject
challenge not required + claims accepted -> reject
```

---

## 4.4 Deterministic canonical ordering

Candidates and allocations are ordered by:

```text
service_node_id
evidence_class
content_id
```

Duplicate rows with the same canonical tuple are rejected.

Reordering input candidates produces the same:

```text
allocations
totals
plan_id
serialized plan
```

---

## 4.5 Category-cap enforcement

The Service Node plan derives its category from the evidence-class enum.

The caller cannot select the reward category.

The plan calculates the available `node_delivery` category pool through:

```text
economics.effective_category_pool_cap(...)
```

This means category basis points and absolute category ceilings now directly affect actual Service Node allocations.

---

## 4.6 Proportional allocation

Within a category, each candidate receives a deterministic floor allocation based on:

```text
category pool
× candidate eligible score
÷ total category eligible score
```

No floating-point math is used.

All multiplication and division use checked integer arithmetic.

---

## 4.7 Per-node cap

Allocations are accumulated by:

```text
service_node_id
```

The total planned amount for one Service Node cannot exceed:

```text
max_reward_minor_per_account_per_epoch
```

The cap applies across multiple evidence classes and content rows for the same node.

---

## 4.8 Per-content cap

Allocations are accumulated by:

```text
content_id
```

The total planned reward associated with one content object cannot exceed:

```text
max_reward_minor_per_content_per_epoch
```

The cap applies across multiple Service Nodes serving or proving work for the same content.

---

## 4.9 Per-node event cap

The plan sums:

```text
evidence_count
```

for each Service Node.

A Service Node is rejected when its total exceeds:

```text
max_events_per_account_per_epoch
```

This prevents splitting excessive activity across multiple content rows to evade the event-count limit.

---

## 4.10 Residual handling

Any amount not allocated because of:

```text
floor rounding
category caps
node caps
content caps
```

remains in:

```text
residual_minor_units
```

The current plan does not redistribute capped or rounded residuals to later candidates.

This keeps allocation deterministic and conservation-safe.

The configured remainder sink is retained in the economics projection, while actual treasury, burn, or stability-buffer mutation remains outside rewarder authority.

---

## 4.11 Plan identity

The Service Node plan receives a deterministic:

```text
plan_id = b3:<64 lowercase hex>
```

The hash binds:

```text
schema
version
epoch ID
accounting snapshot CID
economics hash
policy hash
totals
allocations
non-authority flags
```

The `plan_id` field itself is cleared before hashing to avoid recursive identity.

Changing category caps or other bound plan inputs changes the plan identity.

---

# 5. Canonical accounting epoch handoff

## 5.1 Real `ron-accounting` snapshot consumption

The new adapter consumes:

```text
AccountingEpochSnapshotV1
```

from `ron-accounting`.

It canonicalizes and validates the snapshot before producing reward-planning material.

The rewarder no longer needs to invent a separate Phase 14 accounting snapshot schema.

---

## 5.2 Accounting artifact CID

The adapter computes the accounting artifact identity using:

```text
canonical_accounting_epoch_snapshot_artifact_cid
```

The resulting CID is copied into:

```text
AccountingEpochRewardPlanningHandoffV1
ServiceNodeRewardPlanInput
UserVerificationPointPlanV1
```

This binds all downstream planning material to the same canonical accounting artifact.

---

## 5.3 Economics-binding validation

The handoff verifies that the accounting snapshot’s economics binding matches the selected rewarder economics projection.

The following must match:

```text
economics schema
economics version
economics profile
economics_config_hash
```

Any mismatch fails closed before reward-plan construction.

---

## 5.4 Policy validation

The supplied reward policy must pass existing centralized reward-policy validation.

For this Phase 14 node-reward path, the policy must use:

```text
protocol_pool
```

funding provenance.

The available planning pool is capped by:

```text
caller-supplied available pool
policy max payout
economics epoch pool cap
```

---

## 5.5 Service evidence aggregation

Accepted Service Node snapshot rows are aggregated by:

```text
service_node_id
evidence class
content_id
```

Each accepted accounting row contributes:

```text
one evidence event
one neutral eligible-score point
```

This avoids inventing a hidden hardcoded reward rate inside the adapter.

The aggregated row becomes a `ServiceNodeRewardCandidate` with:

```text
evidence_verified = true
accounting_accepted = true
policy_gate_passed = true
```

Those values are trusted because they are derived from an already validated and classified accounting snapshot, not directly from a node request.

---

## 5.6 Policy-only evidence rejection

The adapter rejects:

```text
PolicyRefusal
ModerationAction
```

from the Service Node reward-planning lane.

These rows may exist for policy, auditing, moderation, or challenge purposes, but cannot become direct reward candidates.

---

# 6. User Node verification planning

## 6.1 Neutral planning points

User Node verification snapshot rows are converted into:

```text
UserVerificationPlannedPointV1
```

Each accepted accounting row contributes exactly:

```text
planned_points = 1
```

The point retains:

```text
sequence
user_node_id
verification_kind
evidence_id
subject_ref
input_digest
```

---

## 6.2 No hardcoded ROC conversion rate

The User Node point plan explicitly records:

```text
reward_amount_assigned = false
```

This session did not introduce an arbitrary:

```text
ROC per verification
ROC per replay
ROC per challenge
ROC per evidence row
```

conversion constant.

A future economics-profile field must define any actual User Node point-to-ROC conversion.

This preserves the rule that mutable economics live in the canonical economics profiles rather than in runtime Rust constants.

---

## 6.3 User Node event limits

User Node verification rows are counted by:

```text
user_node_id
```

The handoff rejects a User Node whose accepted verification rows exceed:

```text
max_events_per_account_per_epoch
```

This applies the same economics-owned anti-farming posture to User Node planning material.

---

## 6.4 Deterministic point-plan identity

The User Node point plan receives its own deterministic BLAKE3 identity.

It binds:

```text
accounting snapshot CID
economics configuration hash
canonical ordered points
total point count
non-authority posture
```

Reordered accounting rows produce the same point plan and plan identity.

---

# 7. Authority and safety boundaries preserved

The new Phase 14 surfaces explicitly preserve:

```text
planning_only = true
registry_resolution_required = true
payout_authority = false
payout_executed = false
wallet_mutation = false
ledger_mutation = false
receipt_created = false
balance_truth = false
```

The accounting handoff also records:

```text
accounting_snapshot_verified = true
policy_gate_passed = true
planning_only = true
payout_authority = false
wallet_mutation = false
ledger_mutation = false
```

User Node point plans record:

```text
planning_only = true
reward_amount_assigned = false
payout_authority = false
wallet_mutation = false
ledger_mutation = false
```

---

## 7.1 Recipient resolution remains external

Service Node plans retain:

```text
service_node_id
```

They do not resolve or accept:

```text
@username
wallet destination
payout account
operator address
```

The plan sets:

```text
registry_resolution_required = true
```

A later trusted `svc-registry` binding path must resolve the Service Node identity to the operator’s external CrabLink payout identity.

This preserves:

```text
anti-recipient-smuggling
anti-self-pay posture
separation between evidence identity and payout destination
```

---

## 7.2 No direct economic mutation added

This session did not add:

```text
direct ROC issuance
direct wallet mutation
direct ledger mutation
receipt creation
balance mutation
epoch finality
quorum acceptance
ROX minting
ROX burning
Solana RPC calls
bridge settlement
staking
liquidity
exchange behavior
```

The authoritative value path remains:

```text
verified evidence
    ->
ron-accounting
    ->
svc-rewarder deterministic capped planning
    ->
ron-policy and registry gates
    ->
later quorum acceptance
    ->
svc-wallet execution
    ->
ron-ledger durable receipt
```

---

# 8. Test changes and results

## 8.1 Economics and category tests

Target:

```text
internal_roc_beta_phase14d_economics_manifest_binding
```

Changed from:

```text
7 tests
```

to:

```text
11 tests
```

Latest focused result:

```text
11 passed
0 failed
```

---

## 8.2 Anti-farming tests

Target:

```text
internal_roc_beta_phase5_antifarming_event_gates
```

Changed from:

```text
5 tests
```

to:

```text
7 tests
```

Latest focused result:

```text
7 passed
0 failed
```

---

## 8.3 Service Node reward-plan tests

New target:

```text
internal_roc_beta_phase14d_service_node_reward_plan
```

Latest focused result:

```text
8 passed
0 failed
```

---

## 8.4 Accounting epoch handoff tests

New target:

```text
internal_roc_beta_phase14d_accounting_epoch_handoff
```

Latest focused result:

```text
6 passed
0 failed
```

---

## 8.5 Existing regression targets retained

The following previously existing behaviors remained green during focused verification:

```text
Phase 5 config-driven planning
Phase 5 policy-gate interlock
Phase 14D economics binding
existing deterministic manifest behavior
existing non-authority boundaries
existing anti-farming class gates
```

---

## 8.6 Compile checks

The latest focused checks passed for:

```text
cargo check -p ron-accounting
cargo check -p svc-rewarder
```

Earlier in this session, before the final accounting handoff was added, the broader milestone gate also passed:

```text
cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder
cargo clippy -p svc-rewarder --all-targets --no-deps -- -D warnings
cargo check -p ron-policy
cargo check -p ron-accounting
cargo check -p svc-rewarder
cargo check --workspace
```

---

# 9. Current verification status

The implementation and focused Phase 14 tests are green.

The final all-crate Phase 14 exit gate was prepared but had not yet been shown as executed at the time these notes were written.

The remaining verification command set is:

```bash
cargo fmt -p svc-rewarder -p ron-accounting -p ron-policy -- --check

cargo test -p svc-rewarder
cargo test -p ron-accounting
cargo test -p ron-policy

cargo clippy -p svc-rewarder --all-targets --no-deps -- -D warnings
cargo clippy -p ron-accounting --all-targets --no-deps -- -D warnings
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings

cargo check --workspace
```

Phase 14 should only be marked formally complete after that final gate returns green.

---

# 10. Current limitations and intentionally deferred work

## 10.1 Registry recipient resolution

The Service Node plan retains `service_node_id`, but does not yet resolve it into an external payout identity.

Future work must use a trusted registry binding:

```text
service_node_id
    ->
registered operator payout @username/account
```

The rewarder must not accept that destination directly from evidence or from an untrusted caller.

---

## 10.2 User Node point-to-ROC conversion

User Node verification rows currently produce neutral deterministic planning points only.

A future complete economics profile must define the monetary conversion behavior before those points can become ROC allocations.

Do not hardcode this conversion in `svc-rewarder`.

---

## 10.3 Category model

All currently rewardable Service Node evidence classes map to:

```text
node_delivery
```

Future category expansion must originate from the shared `ron-policy` economics schema and trusted accounting classifications.

Callers must not supply arbitrary category strings.

---

## 10.4 Residual redistribution

The Service Node planner leaves cap- and rounding-generated residuals unallocated.

It does not currently perform iterative redistribution after a node or content cap is reached.

Any future redistribution rule must be:

```text
deterministic
integer-only
economics-configured
order-independent or canonically ordered
conservation-safe
covered by adversarial tests
```

---

## 10.5 Existing generic reward manifest path

The older account-based `AccountingSnapshot` and generic `RewardManifest` path remains for compatibility.

The new Phase 14 Service Node plan is a dedicated identity-bound planning surface and has not yet replaced every existing HTTP compute or settlement route.

Future integration must not convert `service_node_id` directly into a wallet recipient without trusted registry resolution.

---

# 11. Net result

Before this session, `svc-rewarder` had deterministic generic account-based reward planning and economics identity binding, but it did not have a complete Phase 14 path for:

```text
category-cap enforcement
economics event-count enforcement
service_node_id retention
content-cap enforcement
canonical ron-accounting epoch snapshots
User Node verification planning points
recipient-free node reward candidates
```

After this session, the crate has:

```text
real category-cap calculations
actual category-capped Service Node allocations
per-node reward caps
per-content reward caps
economics-owned event limits
deterministic Service Node plan identities
canonical accounting snapshot CID binding
strict economics profile/hash binding
policy-only evidence rejection
neutral User Node verification point plans
recipient and amount smuggling rejection
explicit non-authority posture
focused green Phase 14 tests
```

The resulting Phase 14 flow is:

```text
typed node evidence
    ->
ron-accounting deterministic classification
    ->
canonical AccountingEpochSnapshotV1
    ->
svc-rewarder verified accounting handoff
    ->
recipient-free Service Node candidates
    ->
economics-bound capped Service Node reward plan
    ->
future trusted registry recipient resolution
    ->
future quorum and wallet execution phases
```

This completes the major `svc-rewarder` implementation work required for Phase 14, subject to the final full regression, strict Clippy, and workspace exit gate.

These notes are ready to paste into the crate changelog, `ALLNOTES.md`, or the next-session carryover record.


### END NOTE - JULY 13 2026 - 14:10 CST




### BEGIN NOTE - JULY 13 2026 - 20:40 CST

Below is the complete carry-over document for the next BUILD_PLAN_Z session.

# RustyOnions / CrabLink — BUILD_PLAN_Z Carry-Over Notes

## Session closeout: Phase 17 completed

Date context:

```text
July 2026
```

Repository:

```text
/Users/mymac/Desktop/RustyOnions
```

Active workstream:

```text
RustyOnions / CrabLink node layer
BUILD_PLAN_Z
```

Current status:

```text
Phase 17 — User-Node Epoch Replay and Fraud Challenges
COMPLETE / GREEN / PARKED
```

Next active phase:

```text
Phase 18 — Sybil Resistance and Protocol-Earned Eligibility
```

---

# 1. Important next-session attachments

At the beginning of the next session, the user plans to attach a newer:

```text
ALLNOTES.md
```

That file contains the accumulated running notes from the changed crates and earlier BUILD_PLAN_Z sessions combined into one document.

Treat the newly attached `ALLNOTES.md` as the main historical carry-over reference.

The next session may also include a newer:

```text
CODEBUNDLE_RS.md
```

Use the newest codebundle as the source of truth for exact source shapes before preparing patches.

Do not rely on stale snippets if the newer codebundle differs.

---

# 2. No GitHub or Git operations

The user does not want anything pushed to GitHub yet.

Do not provide or request commands involving:

```text
git add
git commit
git push
git pull
git fetch
git merge
git rebase
git checkout
git switch
git reset
git clean
GitHub CLI commands
pull request creation
branch creation
tag creation
```

Do not suggest committing “for safety” or creating a temporary branch.

Work directly in the current local repository using targeted file patches and focused Cargo verification.

This remains the rule unless the user explicitly reverses it.

---

# 3. Updated patch-and-test response workflow

The user has explicitly requested that future patch responses include both:

1. the mutation patch
2. a separate Bash block containing every focused command needed to verify that patch

The user should not need to send another message merely to ask for the test commands.

Correct future response structure:

```text
brief explanation of the exact behavior being added or fixed

Patch:
~~~~bash
<mutation commands only>
~~~~

Verification:
~~~~bash
unsetopt nounset
cd /Users/mymac/Desktop/RustyOnions

cargo fmt ...
cargo test ...
cargo clippy ...
cargo check ...
~~~~
```

The patch and verification commands should remain in separate Bash blocks, but both blocks should appear in the same assistant response.

Do not mix the mutation commands and Cargo tests inside one shell block.

When a command fails:

```text
stop expanding scope
inspect the first compiler/test failure
patch only that failure
provide the focused verification block in the same response
```

Do not jump ahead to later phases while the current slice is red.

---

# 4. Development posture to preserve

Continue using a QuickChain-style implementation rhythm:

```text
real Rust behavior
small focused patches
focused tests beside behavior
compile checks
strict Clippy
fix first failure
then expand
```

Do not turn BUILD_PLAN_Z back into:

```text
planning-only work
placeholder scaffolding
broad decision gates
documentation theater
fake success output
closeout documents before implementation
```

Comments should explain:

```text
what the code does
what it validates
what failures mean
what authority it does not have
what the matching tests prove
```

Avoid repeated boilerplate disclaimers in every function.

---

# 5. Final session result

This session completed BUILD_PLAN_Z Phase 17 at both the implementation and regression levels.

The Phase 17 exit gate was:

```text
Regular user nodes can audit the economy and challenge invalid epochs.
```

That gate is now satisfied.

The final regression sequence was:

```bash
cargo test -p ron-proto
cargo test -p svc-wallet
cargo check --workspace
```

All three completed successfully.

The final output showed:

```text
ron-proto full test suite: passed
svc-wallet full test suite: passed
cargo check --workspace: passed
```

The workspace check successfully included the major affected and dependent crates, including:

```text
ron-proto
ron-policy
ron-ledger
ron-app-sdk
ron-audit
micronode
svc-wallet
svc-storage
svc-dht
svc-rewarder
svc-passport
macronode
svc-overlay
svc-gateway
omnigate
svc-registry
```

No compiler or test failure remained at session close. 

---

# 6. Crates changed in this session

Exactly three crates were directly changed:

```text
ron-proto
svc-wallet
micronode
```

No other crate was directly modified in this session.

Dependent crates compiled during workspace verification, but that does not mean their source was changed.

---

# 7. `ron-proto` changes completed

## 7.1 Primary purpose

`ron-proto` became the canonical owner of Service Node epoch-signature message construction.

Previously, `svc-wallet` locally constructed the bytes used to verify quorum signatures.

Phase 17 required `micronode` to verify the same signatures independently.

Allowing both crates to maintain separate serializers could create security-relevant drift.

The solution was to centralize the signed message in `ron-proto`.

## 7.2 Production file changed

```text
crates/ron-proto/src/service_node/epoch_transition.rs
```

## 7.3 Test file changed

```text
crates/ron-proto/tests/internal_roc_beta_phase15_epoch_transition.rs
```

## 7.4 Shared helper added

The public helper is:

```rust
pub fn service_node_signature_message_bytes(
    signature: &ServiceNodeSignatureV1,
) -> Result<Vec<u8>, serde_json::Error>
```

It serializes the exact protocol message that Service Nodes sign for an epoch transition.

## 7.5 Canonical signed fields

The canonical message contains:

```text
domain
version
chain_id
epoch_id
service_node_id
key_id
algorithm
transition_hash
```

The exact domain separator is:

```text
rustyonions.service-node-epoch-signature.v1
```

This binds the signature to:

```text
the RustyOnions signature domain
protocol version
chain/environment
specific epoch
specific Service Node
specific logical key
specific signature algorithm
specific epoch-transition hash
```

## 7.6 `signature_wire` exclusion

The resulting signature bytes are not included in their own preimage.

The canonical message explicitly excludes:

```text
signature_wire
```

Two otherwise identical signature DTOs with different `signature_wire` values produce identical canonical message bytes.

This was locked with a focused regression test.

## 7.7 Protocol boundary preserved

The helper does not:

```text
resolve keys
verify signatures
accept algorithms
establish quorum
accept an epoch
accept a challenge
mutate wallet state
mutate ledger state
claim consensus
claim finality
```

`ron-proto` owns deterministic protocol bytes, not cryptographic or economic authority.

## 7.8 Compile repair

The first `ron-proto` compilation attempt failed because the new helper referenced types that were not imported:

```text
ServiceNodeSignatureV1
SignatureAlg
```

The exact imports were added.

No DTO or schema shape was changed.

## 7.9 New test

The Phase 15 transition suite gained:

```text
service_node_signature_message_is_canonical_and_excludes_wire
```

This proves:

```text
canonical bytes are deterministic
the exact domain separator is present
signature_wire is absent
different signature_wire values do not change the preimage
```

## 7.10 Final verification

Focused transition suite:

```text
12 passed
0 failed
```

Service Node quorum suite remained green:

```text
9 passed
0 failed
```

Strict Clippy passed:

```bash
cargo clippy -p ron-proto \
  --all-targets \
  --no-deps \
  -- -D warnings
```

Focused check passed:

```bash
cargo check -p ron-proto
```

Full crate suite passed:

```bash
cargo test -p ron-proto
```

Workspace check passed.

---

# 8. `svc-wallet` changes completed

## 8.1 Primary purpose

`svc-wallet` was migrated from a locally defined Service Node signature preimage to the canonical helper now owned by `ron-proto`.

The wallet execution design itself was not rewritten.

## 8.2 Production file changed

```text
crates/svc-wallet/src/epoch_execution.rs
```

## 8.3 Shared helper delegation

The wallet imports the protocol helper under a clear alias:

```rust
service_node_signature_message_bytes
    as canonical_signature_message_bytes
```

The wallet’s existing public helper remains available:

```rust
pub fn service_node_signature_message_bytes(
    signature: &ServiceNodeSignatureV1,
) -> WalletResult<Vec<u8>>
```

It now:

```text
enforces the wallet's Ed25519-only rule
delegates canonical byte construction to ron-proto
maps protocol serialization failures into WalletError
```

## 8.4 Public compatibility preserved

The existing wallet helper was retained so current callers and tests did not need a breaking API migration.

The wallet still returns:

```text
WalletResult<Vec<u8>>
```

rather than exposing a raw `serde_json::Error`.

## 8.5 Duplicate serializer removed

The local wallet-only signature preimage structure was removed.

This eliminates the possibility that:

```text
svc-wallet verifies one message format
micronode audits another message format
```

Both now use:

```text
ron_proto::service_node_signature_message_bytes
```

## 8.6 Wallet-owned policy preserved

The wallet still rejects non-Ed25519 quorum signatures.

Moving serialization into `ron-proto` did not move acceptance policy.

Correct separation:

```text
ron-proto:
  canonical bytes

svc-wallet:
  accepted algorithm
  key resolution
  encoding validation
  cryptographic verification
  mutation gating
```

## 8.7 Verification behavior preserved

The wallet still verifies:

```text
declared algorithm is Ed25519
logical key reference resolves
resolved key is Ed25519
signature wire is exactly 128 lowercase hex characters
decoded signature is exactly 64 bytes
signature verifies against canonical message bytes
verification succeeds before ledger mutation
```

## 8.8 Execution order preserved

The wallet still performs:

```text
transition validation
quorum signature verification
recipient-resolution reconstruction
exact prepared-operation comparison
wallet/ledger payout execution
```

An invalid signature still rejects before:

```text
wallet mutation
ledger append
receipt creation
replay-state advancement
```

## 8.9 Recipient-binding behavior preserved

The wallet still rejects:

```text
missing recipient resolution
unresolved recipient state
wrong epoch
wrong registry root
wrong reward-binding root
wrong binding ID
caller-selected recipient account
prepared operation mismatch
```

Service evidence still cannot redirect a payout.

## 8.10 Economics binding preserved

The wallet still validates:

```text
policy hash
economics_config_hash
accounting snapshot hash
reward plan hash
registry root
reward-binding root
transition identity
allocation details
```

The wallet does not define or hardcode ROC reward rates.

Mutable economics remain centralized under the shared economics configuration design.

## 8.11 Compile regression fixed

Removing the old signature serializer also removed:

```rust
use serde::Serialize;
```

That import was still required by the separate deterministic payout operation identity structure:

```rust
#[derive(Serialize)]
struct OperationIdentity<'a>
```

The compiler reported:

```text
cannot find derive macro Serialize
OperationIdentity does not implement Serialize
```

The `serde::Serialize` import was restored.

This did not change the operation identity format.

## 8.12 Phase 16 focused suite

The Phase 16 quorum execution suite passed:

```text
11 passed
0 failed
```

The passing tests covered:

```text
registry root mismatch rejection
recipient binding mismatch rejection
single-node quorum rejection
required economics-config hash
economics-config mismatch rejection
policy mismatch rejection
reward-binding root mismatch rejection
invalid signature rejection before mutation
caller-prepared recipient mismatch rejection
exact retry without double issue
valid quorum atomic execution and replay
```

## 8.13 Final verification

Strict Clippy passed:

```bash
cargo clippy -p svc-wallet \
  --all-targets \
  --no-deps \
  -- -D warnings
```

Focused check passed:

```bash
cargo check -p svc-wallet
```

Full crate suite passed:

```bash
cargo test -p svc-wallet
```

Workspace check passed.

---

# 9. `micronode` changes completed

`micronode` received the largest implementation surface in this session.

Its Phase 17 work was divided into four behavior slices:

```text
17A — deterministic local epoch replay
17B — canonical invalid-epoch challenge construction
17C — real quorum signature verification
17D — bounded challenge submission outbox
```

---

# 10. Phase 17A — Deterministic local economic audit

## 10.1 New production file

```text
crates/micronode/src/economic_audit.rs
```

## 10.2 Module role

The module allows a regular user node to independently review a ROC epoch transition.

It is read-only and non-authoritative.

It does not:

```text
issue ROC
transfer ROC
burn ROC
change balances
append ledger records
produce accepted payout receipts
accept an epoch
claim finality
quarantine a Service Node
punish a signer
```

## 10.3 Replay observation contract

A versioned user-node observation surface was added.

Canonical schema:

```text
micronode.user-node-epoch-replay-observation.v1
```

Version:

```text
1
```

The observation carries independently replayed or reviewed material such as:

```text
accounting snapshot identity
reward-plan identity
policy hash
economics configuration hash
registry root
reward-binding root
applied allocation identities
replayed allocation total
supply-conservation material
```

## 10.4 Structured review result

The local audit returns a structured review with:

```text
reviewed transition identity
accepted/rejected local status
stable finding list
explicit evidence-only posture
```

A rejected review may contain multiple findings.

Equivalent inputs produce equivalent finding order.

## 10.5 Root and binding checks

The audit checks:

```text
accounting root/artifact binding
reward-plan root
policy hash
economics_config_hash
registry root
reward-binding root
transition identity
```

## 10.6 Replay checks

The audit detects:

```text
missing payout application
unexpected payout application
duplicate payout application
allocation identity reuse
replayed total mismatch
transition total mismatch
```

## 10.7 Reward cap review

The audit verifies:

```text
planned total does not exceed reviewed cap
replayed total matches transition total
cap and replay material agree
```

It does not define the reward rate or cap.

## 10.8 Supply conservation

The audit verifies the claimed supply transition against the applied epoch movement.

It rejects unaccounted creation or destruction of ROC.

It does not repair or mutate supply.

## 10.9 Eligibility review

The audit checks that quorum signers correspond to the reviewed eligibility set.

It rejects:

```text
missing signer eligibility
noneligible signer status
signer identity mismatch
key-binding mismatch
wrong transition binding
```

---

# 11. Phase 17B — Canonical invalid-epoch challenges

## 11.1 Challenge builder

The module added:

```rust
build_invalid_epoch_challenge
```

It converts a rejected user-node review into:

```rust
InvalidEpochChallengeV1
```

## 11.2 Accepted review protection

An accepted review cannot be used to fabricate a challenge.

This is explicitly tested.

## 11.3 Exact transition binding

A review for one transition cannot be reused against another transition.

The challenge binds:

```text
chain ID
epoch ID
transition hash
challenger ID
primary challenge kind
evidence hash
submission timestamp
```

## 11.4 Deterministic BLAKE3 evidence

The complete review evidence is serialized deterministically and hashed using BLAKE3.

Evidence ID shape:

```text
b3:<64 lowercase hexadecimal characters>
```

Challenge ID is deterministically derived from:

```text
epoch identity
evidence digest
```

Equivalent invalid reviews produce identical challenge and evidence identities.

## 11.5 Stable primary precedence

A review can contain multiple findings.

The challenge DTO carries one primary challenge kind.

A fixed precedence rule selects the primary kind while the full finding set remains committed into the evidence hash.

This prevents map or iteration ordering from changing the challenge route.

## 11.6 Non-authority preserved

Constructing a challenge does not mean:

```text
challenge accepted
epoch reversed
signer removed
node quarantined
reward clawed back
penalty imposed
finality changed
```

---

# 12. Phase 17C — Real Ed25519 quorum verification

## 12.1 Cargo dependencies added

`micronode` gained:

```text
ron-kms
hex
```

These support actual local cryptographic verification.

## 12.2 Key resolver trait

The audit module added:

```rust
pub trait EpochQuorumKeyResolver {
    fn resolve_key(&self, key_ref: &str) -> Option<KeyId>;
}
```

This maps a logical Service Node key reference to a concrete KMS key identity.

The resolver does not:

```text
create keys
rotate keys
sign messages
change registry state
change eligibility
accept challenges
```

## 12.3 Signed review function

The module added:

```rust
review_epoch_transition_with_signatures
```

It combines:

```text
structural/replay audit
real cryptographic signature verification
```

It reuses the existing audit path rather than creating a second competing validator.

## 12.4 Canonical message reuse

`micronode` now calls:

```rust
ron_proto::service_node_signature_message_bytes
```

This guarantees byte-for-byte parity with `svc-wallet`.

## 12.5 Algorithm enforcement

Phase 17 currently accepts only:

```text
Ed25519
```

The verifier checks both:

```text
declared signature algorithm
resolved KMS key algorithm
```

## 12.6 Signature-wire enforcement

The audit requires:

```text
exactly 128 characters
lowercase hexadecimal only
exactly 64 decoded bytes
```

It rejects malformed, uppercase, short, oversized, or non-hex signatures.

## 12.7 Real verification

The audit uses the `ron-kms` verifier against:

```text
resolved key
canonical message bytes
decoded signature
```

A signature string being present is not treated as proof.

## 12.8 Signature challenge mapping

Failures map into the canonical invalid Service Node signature challenge path.

This includes:

```text
tampered signature
unknown key reference
unsupported algorithm
wrong resolved key type
malformed signature wire
cryptographic verification failure
```

## 12.9 Real-signature tests

The focused suite uses `ron_kms::MemoryKeystore` to create real Ed25519 keys and signatures.

Tests added:

```text
user_node_accepts_real_ed25519_quorum_signatures
tampered_ed25519_signature_requires_canonical_challenge
unresolved_quorum_key_requires_signature_challenge
```

---

# 13. Phase 17D — Bounded challenge outbox

## 13.1 New production file

```text
crates/micronode/src/challenge_outbox.rs
```

## 13.2 Purpose

The outbox provides a real local handoff between:

```text
canonical challenge constructed
```

and:

```text
configured transport acknowledges receipt
```

It does not implement challenge adjudication.

## 13.3 Module export

`crates/micronode/src/lib.rs` was updated to expose:

```rust
pub mod economic_audit;
pub mod challenge_outbox;
```

## 13.4 Capacity bounds

Default retained records:

```text
128
```

Maximum items returned by one recent-record read:

```text
64
```

Zero capacity is rejected.

Invalid read limits are rejected.

## 13.5 Submission lifecycle

Each record has one local transport state:

```text
queued
dispatching
submitted
```

Meaning:

```text
queued:
  waiting for transport

dispatching:
  synchronous handoff in progress

submitted:
  configured transport acknowledged receipt
```

`submitted` does not mean challenge accepted.

## 13.6 Truthful record fields

Each record includes:

```text
sequence
canonical challenge
submission state
attempt count
optional submission reference
optional acknowledgement timestamp
```

Explicit truth flags remain:

```text
evidence_only = true
challenge_accepted = false
finality_claimed = false
wallet_mutation = false
ledger_mutation = false
```

## 13.7 Submission sink trait

The module defines a narrow transport boundary:

```rust
InvalidEpochChallengeSubmissionSink
```

The sink can only return:

```text
submission_ref
acknowledged_at_ms
```

It cannot return protocol acceptance through this interface.

## 13.8 Duplicate suppression

The outbox rejects duplicate deterministic challenge IDs.

The same invalid epoch review cannot create an unbounded number of identical queued records.

## 13.9 Fail-closed capacity behavior

At capacity:

```text
queued records are not silently evicted
dispatching records are not silently evicted
enqueue fails if all retained records are still pending
```

A previously transport-acknowledged record may be evicted to make space for new pending evidence.

## 13.10 Retry behavior

When a sink fails:

```text
attempt count increments
record returns to queued
challenge identity remains unchanged
no fake submission reference appears
record remains retryable
```

## 13.11 Acknowledgement validation

The outbox validates:

```text
nonempty bounded submission reference
allowed lowercase reference characters
nonzero acknowledgement timestamp
acknowledgement timestamp not earlier than challenge timestamp
```

Malformed acknowledgement returns the record to queued.

## 13.12 Process-local scope

The outbox is currently:

```text
bounded
in-process
nonpersistent
not automatically wired to the running scheduler
not connected to live mailbox/overlay/HTTP submission
```

Do not describe it as a complete distributed challenge system.

---

# 14. Phase 17 focused test progression

The focused test file is:

```text
crates/micronode/tests/internal_roc_beta_phase17_epoch_replay.rs
```

The suite grew through the session:

```text
initial structural/replay behavior: 6 tests
after challenge behavior: 10 tests
after real signatures: 13 tests
after challenge outbox: 18 tests
```

Final result:

```text
18 passed
0 failed
```

Final tests:

```text
accepted_review_cannot_fabricate_invalid_epoch_challenge
identical_invalid_review_builds_deterministic_evidence_identity
multi_finding_challenge_uses_stable_primary_precedence
failed_transport_handoff_remains_queued_for_retry
deterministic_challenge_duplicate_is_not_queued_twice
rejected_review_builds_canonical_invalid_epoch_challenge
pending_challenge_is_never_silently_evicted_at_capacity
rejected_review_queues_truthful_challenge_submission_record
successful_transport_handoff_does_not_claim_challenge_acceptance
user_node_accepts_valid_structural_epoch_replay
user_node_detects_economics_configuration_hash_mismatch
user_node_detects_cap_and_replay_total_mismatch
tampered_ed25519_signature_requires_canonical_challenge
user_node_detects_duplicate_payout_in_replay
user_node_detects_invalid_or_noneligible_quorum_signer
user_node_detects_supply_conservation_mismatch
unresolved_quorum_key_requires_signature_challenge
user_node_accepts_real_ed25519_quorum_signatures
```

---

# 15. Compile failures encountered and fixed

These failures are useful context so the next session does not repeat the same mistakes.

## 15.1 `ron-proto` missing imports

Failure:

```text
cannot find ServiceNodeSignatureV1
cannot find SignatureAlg
```

Fix:

```text
import ServiceNodeSignatureV1
import SignatureAlg
```

## 15.2 `svc-wallet` missing Serialize derive import

Failure:

```text
cannot find derive macro Serialize
OperationIdentity does not implement Serialize
```

Cause:

```text
removal of the old local signature serializer also removed a still-needed serde import
```

Fix:

```rust
use serde::Serialize;
```

## 15.3 `micronode` production missing imports

Failure:

```text
cannot find SignatureAlg
cannot find service_node_signature_message_bytes
```

Fix:

```text
import both from ron-proto
```

## 15.4 `micronode` test missing Phase 17C imports

Failure:

```text
cannot find EpochQuorumKeyResolver
cannot find review_epoch_transition_with_signatures
cannot find service_node_signature_message_bytes
```

Fix:

```text
update the exact test import blocks
```

The first scripted import replacement did not alter the file because its search anchor did not match the actual formatted source.

The second patch used the exact current import block and succeeded.

Lesson:

```text
When a scripted replacement unexpectedly has no effect,
inspect the exact current source/codebundle and use a byte-matching anchor.
```

---

# 16. Full `micronode` regression status

Strict Clippy passed:

```bash
cargo clippy -p micronode \
  --all-targets \
  --no-deps \
  -- -D warnings
```

Full crate test suite passed:

```bash
cargo test -p micronode -- --nocapture
```

This included:

```text
10 library unit tests
admin parity
authentication gates
backpressure
CLI smoke
facets loader
facets proxy
guard behavior
18 Phase 17 tests
KV roundtrip
passive runtime tests
documentation tests
```

Focused check passed:

```bash
cargo check -p micronode
```

---

# 17. Phase 17 architectural result

The completed flow is now:

```text
Service Nodes construct/sign quorum transition
        |
        v
ron-proto defines canonical signed bytes
        |
        +--> svc-wallet verifies before mutation
        |
        +--> micronode independently verifies
                    |
                    v
          deterministic epoch replay
                    |
                    v
          accepted local review
                    or
          canonical invalid-epoch challenge
                    |
                    v
          bounded retryable transport outbox
```

Important separation:

```text
micronode:
  verify
  replay
  detect
  challenge
  queue
  hand off evidence

svc-wallet:
  approved ROC mutation front door

ron-ledger:
  durable balance and receipt truth
```

---

# 18. Boundaries that must not regress

## 18.1 No unilateral minting

```text
No single Service Node can mint ROC.
```

A structurally valid transition is not enough.

Mutation still requires:

```text
quorum
reviewed hashes
recipient resolution
wallet execution
ledger recording
```

## 18.2 No node self-pay shortcut

Correct flow:

```text
service evidence references service_node_id
registry resolves the bound recipient
wallet pays canonical account
ledger records receipt
```

Forbidden:

```text
evidence carries arbitrary payout address
node selects a recipient per claim
rewarder pays a raw @ string
node changes recipient during payout
```

## 18.3 Economics configuration remains centralized

All mutable ROC economics must remain under:

```text
configs/roc-economics.toml
configs/roc-economics.dev.toml
```

Do not hardcode reward rates, caps, splits, anti-farming limits, probation reward limits, or Sybil-related economic thresholds across crates.

For Phase 18, any configurable probation caps, new-node limits, or weighting thresholds must be loaded from the canonical economics/policy configuration model rather than scattered as production constants.

Development-only fixed values are acceptable only when clearly test/dev scoped and not presented as production economics.

## 18.4 No fake challenge acceptance

A transport acknowledgement means only:

```text
the configured transport received the DTO
```

It does not mean:

```text
challenge accepted
fraud proven
epoch reversed
signer punished
node quarantined
rewards clawed back
finality changed
```

## 18.5 User-node privacy remains locked

User nodes remain:

```text
outbound-only
loopback-admin-only
not public residential providers
not direct user-to-user TCP listeners
not peer-IP display surfaces
```

Do not weaken this while integrating eligibility, challenge history, or identity descriptors.

## 18.6 No public bridge scope

BUILD_PLAN_Z still excludes:

```text
live ROC ↔ ROX settlement
live Solana calls
public ROX minting
staking
liquidity
exchange behavior
production deployment
```

ROX Anchor remains a separate workstream.

## 18.7 `svc-admin` remains optional

`svc-admin` is not required for Service Node runtime.

It may later display eligibility or quarantine state, but it must not own that state or become the mandatory control plane.

## 18.8 Existing challenge outbox is process-local

Do not accidentally describe or treat the new outbox as durable network submission.

Future runtime wiring must explicitly decide:

```text
where challenge transport goes
whether queue persistence is required
how retry survives process restart
how acknowledgement is authenticated
how adjudication state is observed
```

Those features were not completed in Phase 17.

---

# 19. BUILD_PLAN_Z completion position

BUILD_PLAN_Z contains phases:

```text
Phase 0 through Phase 24
```

Completed through:

```text
Phase 17
```

Next:

```text
Phase 18
```

By simple phase count:

```text
18 completed phases out of 25 total phases
approximately 72% by phase count
```

This is not an effort-weighted percentage.

The remaining phases include substantial cross-crate integration and hardening, so the practical effort remaining may be greater than 28%.

A reasonable project status description is:

```text
BUILD_PLAN_Z core node, content, evidence, rewards,
quorum execution, and user-node audit foundations
are implemented through Phase 17.

Remaining work is centered on:
  objective eligibility
  Sybil resistance
  quarantine
  operator UX integration
  end-to-end devnet proof
  chaos/hardening
  private beta readiness
```

---

# 20. Next active phase: Phase 18

## Phase title

```text
Phase 18 — Sybil Resistance and Protocol-Earned Eligibility
```

## Goal

```text
Prevent attackers from spinning up many fresh Service Nodes
to control quorum.
```

BUILD_PLAN_Z requires:

```text
service-node identity descriptor
reward recipient binding
candidate/probation/eligible/degraded/quarantined/blocked states
service history root
challenge history
new-node caps
quorum weighting
requester/provider diversity rules
```

Required tests:

```text
new node starts as candidate/probation
candidate cannot control quorum
probation rewards capped
service history promotes eligibility
many fresh nodes cannot exceed threshold
quarantined node cannot sign valid quorum
manual founder-approved trust flag does not exist
```

Exit gate:

```text
Permissionless participation has objective protocol gates.
```

These are the controlling Phase 18 requirements. 

---

# 21. Recommended Phase 18 implementation strategy

Do not attempt all of Phase 18 in one patch.

Use small slices.

## Phase 18A — Canonical eligibility lifecycle foundation

Start in:

```text
ron-proto
```

Goal:

```text
define the canonical protocol-earned Service Node eligibility lifecycle
without creating a second competing quorum model
```

Reuse the existing Phase 15 foundations:

```text
EpochEligibilityV1
EpochEligibilityStatusV1
EpochQuorumThresholdV1
ServiceNodeQuorumV1
ServiceNodeSignatureV1
ServiceNode reward binding DTOs
```

Do not immediately invent unrelated duplicate types.

First inspect the exact current definitions in:

```text
crates/ron-proto/src/service_node/quorum.rs
crates/ron-proto/src/service_node/epoch_transition.rs
crates/ron-proto/src/service_node/mod.rs
```

Likely Phase 18A behavior should establish or extend:

```text
candidate
probation
eligible
degraded
quarantined
blocked
```

The lifecycle must define objective eligibility effects such as:

```text
candidate:
  may register
  may accumulate service history
  may not control quorum

probation:
  limited participation
  capped reward posture
  reduced or zero quorum weight depending on exact plan rule

eligible:
  may count toward quorum under reviewed threshold rules

degraded:
  reduced or suspended quorum/reward posture
  recoverable through objective service history

quarantined:
  cannot count toward valid quorum
  reward denied while quarantined

blocked:
  cannot participate
  cannot earn rewards
```

Do not finalize these semantics without checking the exact current BUILD_PLAN_Z language and existing DTOs.

## Phase 18B — Service history and challenge history inputs

Likely ownership:

```text
svc-registry:
  canonical Service Node eligibility records
  identity descriptor
  reward binding linkage
  current lifecycle state
  effective epoch
  history roots

ron-proto:
  DTOs and deterministic validation

ron-accounting / ron-audit:
  evidence-derived service and challenge history material

ron-policy:
  objective promotion/degradation gate evaluation
```

Do not let `svc-registry` invent economic rates.

Do not let `ron-policy` mutate registry state directly.

## Phase 18C — Quorum weighting and fresh-node caps

Extend the existing quorum validation path rather than bypassing it.

Required checks include:

```text
candidate signatures cannot satisfy quorum
quarantined signatures cannot satisfy quorum
blocked signatures cannot satisfy quorum
fresh-node population cannot dominate the threshold
duplicate identities cannot increase weight
weight calculation is deterministic
input order does not change the result
```

Any weight or cap values that are mutable economics/policy settings must come from the shared config model.

## Phase 18D — Probation reward cap enforcement

Likely affected crates:

```text
ron-proto
svc-rewarder
ron-policy
svc-wallet
possibly ron-accounting
```

Correct posture:

```text
eligibility status constrains reward planning
reward planning remains non-authoritative
wallet still executes only approved quorum-backed payouts
ledger still records final internal ROC truth
```

Do not let probation state directly mutate a balance.

## Phase 18E — Sybil scenario tests

Required scenarios:

```text
many newly registered nodes remain candidate/probation
candidate signatures do not count as controlling quorum
many fresh nodes cannot satisfy an eligible-node threshold
service history can promote a node deterministically
quarantined node signature is rejected
manual founder-approved or trusted-node field is absent
probation payout cannot exceed configured cap
```

## Phase 18F — Cross-crate regression closeout

Close Phase 18 only after:

```text
focused protocol tests
focused registry/policy tests
focused quorum tests
focused reward cap tests
strict Clippy for changed crates
full changed-crate tests
cargo check --workspace
```

---

# 22. Recommended first action in the next session

The next session should begin by examining the exact current Phase 18 foundations before changing code.

Suggested initial read-only commands:

```bash
unsetopt nounset

cd /Users/mymac/Desktop/RustyOnions

grep -n \
  "Phase 18 — Sybil Resistance and Protocol-Earned Eligibility" \
  BUILD_PLAN_Z.md

rg -n \
  "EpochEligibility|EligibilityStatus|QuorumThreshold|ServiceNodeQuorum|candidate|probation|quarantined|blocked" \
  crates/ron-proto \
  crates/svc-registry \
  crates/ron-policy \
  crates/svc-rewarder \
  crates/svc-wallet
```

Then inspect the exact current files or generate a fresh codebundle before constructing Phase 18A.

The first implementation patch should be narrow and compile-testable.

Do not begin with UI work.

Do not begin with `svc-admin`.

Do not begin with live registry networking.

Start with the canonical protocol/lifecycle foundation.

---

# 23. Suggested next-session baseline verification

Because Phase 17 closed green, a full test rerun is not required before every new patch.

A lightweight baseline is enough:

```bash
unsetopt nounset

cd /Users/mymac/Desktop/RustyOnions

cargo check -p ron-proto
cargo check -p svc-registry
cargo check -p ron-policy
cargo check -p svc-rewarder
cargo check --workspace
```

If a baseline command fails:

```text
fix that first failure before Phase 18 expansion
```

If green:

```text
begin Phase 18A
```

The assistant should include this or an appropriately focused verification block in the same response as the first Phase 18A patch.

---

# 24. Phase 19 thereafter

## Phase title

```text
Phase 19 — Bad Node Detection and Quarantine
```

## Goal

```text
Contain malicious or unreliable nodes.
```

Signals include:

```text
hash mismatch
denylist violation
tombstone violation
fake delivery proof
provider spam
replay attempts
self-traffic loops
challenge failure
invalid epoch proposal
invalid epoch signature
unilateral mint attempt
privacy leak
reward-recipient binding abuse
```

Required tests:

```text
bad hash provider quarantined
denylist violator blocked
privacy leak rejected
invalid epoch signer quarantined
binding abuse blocks rewards
blocked node earns no rewards
appeal status visible
```

Exit gate:

```text
Bad nodes are contained and reward-denied.
```

Phase 19 should build directly on the canonical lifecycle and eligibility state introduced in Phase 18.

Do not create a separate quarantine state machine in `macronode`, `svc-registry`, `ron-policy`, and `micronode`.

There must be one canonical state model.

---

# 25. Phase 20 thereafter

## Phase title

```text
Phase 20 — Optional svc-admin Operator Console
```

Goal:

```text
Make the optional UI useful for moderation and rewards
without requiring it for runtime.
```

The UI should display real backend truth for:

```text
Service Node role/profile/health/readiness
OAP status
amnesia/persistence mode
storage use
bandwidth use
provider records
policy freshness
blocked/pruned count
pending review queue
persistence approvals
reward recipient @ address
reward binding status
delivery evidence count
reward evidence count
pending reward plans
confirmed ledger receipts
quorum eligibility/status
quarantine status
privacy compliance
```

It must not display:

```text
fake ROC
fake receipts
fake finality
Service Node controls on a User Node
cloud-dashboard surfaces by default
public admin exposure as normal
```

Exit gate:

```text
Operators have a useful optional dashboard.
Service Node remains CLI/headless operable.
```

Do not start this phase before eligibility and quarantine truth exist in backend crates.

---

# 26. Phase 21 thereafter

## Phase title

```text
Phase 21 — CrabLink Operator Mode
```

Goal:

```text
Allow CrabLink to optionally manage a Service Node
without fusing app and daemon authority.
```

Planned behavior:

```text
Node Operator Mode
connect to local or remote Service Node
authenticate locally
show status
bind reward @ address
review moderation queue
approve/reject persistence
show confirmed ROC receipts
```

Required boundaries:

```text
CrabLink cannot silently mutate policy
CrabLink does not become wallet or ledger truth
CrabLink does not require a Service Node to run
confirmed rewards appear only after ledger receipt
```

Exit gate:

```text
CrabLink is a friendly optional controller,
not a required daemon container.
```

---

# 27. Phase 22 thereafter

## Phase title

```text
Phase 22 — End-to-End Local Two-Node Devnet Smoke
```

Required scenario:

```text
start CrabLink with User Node
start Service Node quorum fixture
bind Service Node reward recipient
publish content
create source bundle
store and serve over OAP
fetch through privacy-safe path
verify b3
emit delivery proof
user node verifies/challenges sample
accounting snapshots evidence
rewarder creates capped plan
policy gates plan
registry resolves recipient
Service Node quorum accepts epoch
wallet executes payout
ledger records receipt
user node replays epoch
CrabLink shows confirmed ROC
operator surfaces show truthful status
```

Required proof:

```text
User Node active
Service Node headless
optional UI not required
reward recipient bound
b3 verified
OAP used
privacy path valid
evidence accepted
reward plan deterministic
quorum required
single node cannot mint
ledger receipt stable
confirmed ROC only after receipt
```

Exit gate:

```text
Local devnet proves the complete
User Node / Service Node / reward-recipient loop.
```

---

# 28. Phase 23 thereafter

## Phase title

```text
Phase 23 — Hardening, Chaos, and Abuse Drills
```

Required chaos cases include:

```text
Service Node offline
admin UI disabled
setup token expired
User Node paused
provider corrupt bytes
provider serves denylisted content
provider leaks IP
provider replays proof
stale DHT record
cache eviction
missing source bundle
tombstone while serving
reward binding rotation mid-epoch
rewarder unavailable
wallet duplicate payout
ledger replay mismatch
single node tries to mint
Sybil nodes attempt quorum
invalid challenge submitted
privacy relay unavailable
```

Required behavior:

```text
reads degrade
writes fail closed where needed
economic paths fail closed
bad content rejected
bad Service Node quarantined
invalid epoch challenged
Sybil nodes capped/probationed
privacy leak rejected
reward-recipient abuse rejected
no fake success
no fake payout
no fake finality
```

Phase 23 includes strict Clippy across the major node and economic crates.

Use `--no-deps` for focused dependent-crate Clippy during development unless the exact phase gate explicitly requires the whole dependency graph.

---

# 29. Phase 24 thereafter

## Phase title

```text
Phase 24 — Private Beta Readiness
```

Required artifacts include:

```text
private beta node runbook
CrabLink User Node UX guide
CrabLink Service Node operator quickstart
crabnode CLI guide
optional admin UI guide
first-run setup guide
reward @ address binding guide
ROC mining/reward explanation
Service Node quorum explanation
IP privacy explanation
moderation/pruning guide
persistence policy guide
policy/denylist operations guide
incident response guide
known limitations
```

Constraints remain:

```text
bounded ROC values
private/dev configurations
no public ROX/Solana bridge runtime
no staking/liquidity/exchange behavior
no manual trusted-node production doctrine
temporary lab/devnet caps allowed
aggressive telemetry/audit
easy emergency halt
privacy enabled by default
admin UI loopback-only unless explicitly changed
unvetted content amnesia-first
```

Exit gate:

```text
Private beta candidate ready.
```

The remaining BUILD_PLAN_Z phases and their exact intent are defined in the controlling build plan. 

---

# 30. Likely Phase 18 crate ownership

The exact patch sequence must be based on current source, but the likely ownership is:

## `ron-proto`

Owns:

```text
canonical eligibility lifecycle DTOs
identity descriptor fields
service-history root references
challenge-history root references
quorum-weight input shapes
strict validation
wire names
version/schema constants
```

Must not own:

```text
database state
network lookups
policy decisions
wallet mutation
ledger mutation
manual trust
```

## `svc-registry`

Owns:

```text
Service Node descriptor storage
reward recipient binding linkage
current eligibility state
effective epoch
registry roots
history references
rotation/update rules
```

Must not own:

```text
reward rate calculations
wallet payout
ledger mutation
founder-approved trust
```

## `ron-policy`

Owns:

```text
objective promotion/degradation gates
new-node cap policy
probation restrictions
diversity requirements
eligibility decision explanation
```

Must not directly mutate registry or balances.

## `ron-accounting` / `ron-audit`

Likely own:

```text
service history reports
challenge history reports
auditable evidence roots
non-authoritative derived observations
```

## `svc-rewarder`

Likely owns:

```text
probation reward planning limits
eligibility-aware reward planning
deterministic capped plans
```

It must not pay or mutate balances.

## `svc-wallet`

Likely enforces:

```text
transition eligibility root matches reviewed expectation
ineligible/quarantined signers do not satisfy quorum
probation reward cap is not exceeded before execution
```

It remains the mutation front door.

## `micronode`

Likely verifies:

```text
eligibility lifecycle state
service/challenge history roots
quorum weight
fresh-node cap
probation limits
quarantined/blocked signer rejection
```

It remains read-only and challenge-capable.

---

# 31. Phase 18 design hazards to avoid

## Do not duplicate existing types

Phase 15 already introduced eligibility and quorum foundations.

Inspect and extend those before creating:

```text
another eligibility enum
another quorum threshold DTO
another signer-state machine
another registry state type
```

## Do not create manual trust

Forbidden fields or concepts include:

```text
founder_approved
trusted_node
manual_trust_override
admin_whitelisted_for_quorum
always_eligible
bootstrap_mint_authority
```

Development fixtures may seed deterministic eligibility data, but must not create a production trust doctrine.

## Do not make uptime alone equal trust

Protocol-earned eligibility should not be based only on:

```text
node age
process uptime
registration timestamp
```

It must use objective service and challenge history.

## Do not let node count equal quorum power

Many fresh identities must not gain control merely by appearing in the registry.

Fresh nodes must begin in candidate/probation posture.

## Do not hardcode mutable economics

Probation reward caps and related mutable values belong in canonical configuration.

## Do not let eligibility bypass reward binding

A node may be eligible for quorum but still unable to receive a reward if its reward recipient binding is absent or invalid.

## Do not let reward binding imply eligibility

A valid payout binding proves recipient ownership, not good Service Node behavior.

## Do not let challenge count alone become guilt

Challenge history must distinguish:

```text
submitted challenge
transport acknowledged
accepted/adjudicated challenge
rejected challenge
unresolved challenge
```

Phase 17 currently implements only local evidence and transport acknowledgement.

Do not treat all submitted challenges as proven violations.

---

# 32. Recommended first Phase 18 test file

A likely focused `ron-proto` test file would be:

```text
crates/ron-proto/tests/internal_roc_beta_phase18_protocol_eligibility.rs
```

Possible initial tests:

```text
new descriptor starts candidate
candidate cannot contribute quorum weight
probation state has bounded participation fields
eligible state requires service-history binding
quarantined state contributes zero quorum weight
blocked state contributes zero quorum and reward eligibility
lifecycle transition rejects epoch regression
history roots must be valid b3 identifiers
reward-binding identity must remain bound
unknown fields reject
manual trusted-node field rejects
input ordering does not affect deterministic eligibility result
```

Do not create this exact file blindly.

First inspect current test naming and DTO foundations.

---

# 33. Suggested next-session opener

Use this as the opening context in the next session:

```text
We are continuing RustyOnions BUILD_PLAN_Z after completing Phase 17.

Phase 17 is green and parked:
- ron-proto owns canonical Service Node epoch-signature bytes
- svc-wallet uses the shared preimage and remains green
- micronode performs deterministic epoch replay, real Ed25519 quorum verification, canonical InvalidEpochChallengeV1 construction, and bounded retryable challenge transport handoff
- the Phase 17 focused suite is 18/18
- full ron-proto and svc-wallet tests passed
- cargo check --workspace passed

The next active phase is Phase 18:
Sybil Resistance and Protocol-Earned Eligibility.

I am attaching a newer ALLNOTES.md containing the combined running crate notes. Treat it and the newest CODEBUNDLE_RS as source references.

Do not use Git or GitHub commands.
Do not push anything.
Use focused QuickChain-style Rust patches.
After every patch, include a separate Bash block in the same response containing all formatting, test, Clippy, and check commands needed to verify it.

Start by inspecting the current ron-proto eligibility and quorum foundations, then implement the smallest compile-tested Phase 18A lifecycle slice without duplicating existing state machines or introducing manual trusted-node fields.
```

---

# 34. Exact known green commands at closeout

These commands were green by the end of the session:

```bash
cargo test -p ron-proto
cargo test -p svc-wallet
cargo test -p micronode -- --nocapture

cargo clippy -p ron-proto \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo clippy -p svc-wallet \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo clippy -p micronode \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo check -p ron-proto
cargo check -p svc-wallet
cargo check -p micronode
cargo check --workspace
```

Focused Phase 15 transition test:

```bash
cargo test -p ron-proto \
  --test internal_roc_beta_phase15_epoch_transition \
  -- --nocapture
```

Result:

```text
12 passed
```

Focused Phase 16 execution test:

```bash
cargo test -p svc-wallet \
  --test internal_roc_beta_phase16_quorum_execution \
  -- --nocapture
```

Result:

```text
11 passed
```

Focused Phase 17 test:

```bash
cargo test -p micronode \
  --test internal_roc_beta_phase17_epoch_replay \
  -- --nocapture
```

Result:

```text
18 passed
```

---

# 35. Files known to have changed

## `ron-proto`

```text
crates/ron-proto/src/service_node/epoch_transition.rs
crates/ron-proto/tests/internal_roc_beta_phase15_epoch_transition.rs
```

## `svc-wallet`

```text
crates/svc-wallet/src/epoch_execution.rs
```

## `micronode`

```text
crates/micronode/Cargo.toml
crates/micronode/src/economic_audit.rs
crates/micronode/src/challenge_outbox.rs
crates/micronode/src/lib.rs
crates/micronode/tests/internal_roc_beta_phase17_epoch_replay.rs
```

The newest Rust codebundle from this session contains the current repository source snapshot and should be preferred over older snippets when checking exact imports or file layout. 

---

# 36. Session completion statement

The correct one-line closeout is:

```text
BUILD_PLAN_Z is green through Phase 17. Regular User Nodes can now deterministically replay ROC epoch transitions, verify real Service Node Ed25519 quorum signatures using the same canonical ron-proto message as svc-wallet, construct canonical invalid-epoch challenges, and retain retryable truthful submission handoffs without gaining wallet, ledger, consensus, or finality authority. The next active work is Phase 18 protocol-earned eligibility and Sybil resistance.
```

---

# 37. Final reminders for the next assistant

```text
Read the newly attached ALLNOTES.md.
Use the newest codebundle.
Do not use Git commands.
Do not push to GitHub.
Do not ask for test commands in a later turn.
After each patch, include the verification Bash block in the same response.
Start with Phase 18A.
Inspect before patching.
Reuse existing eligibility/quorum types.
Fix the first failure before expanding.
Keep economics config centralized.
Keep user nodes private.
Keep svc-admin optional.
Keep micronode read-only.
Keep svc-wallet as mutation front door.
Keep ron-ledger as durable truth.
Do not invent manual trusted nodes.
Do not claim challenge acceptance or finality.
Do not add live ROX/Solana behavior.
```

These notes are ready to append to the newer `ALLNOTES.md` or save as the dedicated next-session handoff.


### END NOTE - JULY 13 2026 - 20:40 CST



### BEGIN NOTE - JULY 14 2026 - 01:05 CST


Next is the `svc-rewarder` changelog. These notes cover the Phase 19 enforcement-aware reward review added in this session, including denial-only outcomes, honest-candidate continuation, appeal visibility, deterministic hashing, and the final full-crate closeout. 

# `svc-rewarder` Changelog Notes — Phase 19 Enforcement-Aware Reward Denial

## Summary

This session extended `svc-rewarder` with the reward-path enforcement required for BUILD_PLAN_Z Phase 19, **Bad Node Detection and Quarantine**.

The crate can now consume canonical Service Node lifecycle descriptors and registry-owned enforcement statuses, exclude contained nodes from reward planning, emit explicit deterministic denial records, and continue planning rewards for honest eligible or probationary nodes.

The implementation closes the Phase 19 economic requirement that:

* blocked nodes earn no rewards;
* quarantined nodes earn no rewards;
* degraded nodes earn no rewards;
* reward-recipient binding abuse blocks reward planning;
* pending appeals do not restore rewards;
* one bad candidate does not suppress valid reward planning for honest nodes.

The rewarder remains a planning and handoff layer only.

It does not:

* mutate Service Node lifecycle state;
* quarantine or block nodes;
* resolve appeals;
* alter reward bindings;
* execute payouts;
* mutate wallets;
* mutate the ledger;
* create payout receipts;
* mint or burn tokens;
* claim balance or finality truth.

---

# Files Added

## `crates/svc-rewarder/src/core/service_node_enforcement_review.rs`

Added the canonical Phase 19 enforcement-aware reward review module.

This module:

* validates candidate input;
* validates Service Node descriptors;
* validates registry enforcement statuses;
* requires exact candidate-to-descriptor matching;
* requires exact contained-node-to-status matching;
* separates allowed candidates from contained candidates;
* records deterministic denial entries;
* invokes the existing Phase 18 eligibility-aware planner for allowed candidates;
* emits a denial-only review when every candidate is contained;
* preserves non-authority boundaries.

## `crates/svc-rewarder/tests/internal_roc_beta_phase19_enforcement_reward_denial.rs`

Added focused tests covering:

* blocked-node reward exclusion;
* honest-node continuation;
* denial-only reviews;
* reward-recipient binding abuse;
* pending appeals;
* missing enforcement status;
* mismatched enforcement status;
* deterministic behavior under reordered inputs.

---

# Files Updated

## `crates/svc-rewarder/src/core/mod.rs`

Registered and publicly exported the new enforcement-aware reward review surface.

The crate now exports:

* `compute_service_node_reward_review_with_enforcement`
* `ServiceNodeEnforcementRewardReview`
* `ServiceNodeRewardDenialV1`
* `SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_SCHEMA`
* `SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_VERSION`

This allows approved callers such as `svc-registry` integration tests and future coordinator surfaces to use one canonical enforcement-aware planning path.

## `crates/svc-rewarder/src/core/service_node_plan.rs`

Adjusted selected existing validation helpers and constants to crate-internal shared visibility so the new Phase 19 module could reuse the real Phase 14/18 reward validation logic.

The following were exposed only to sibling core modules:

* the maximum Service Node reward-candidate limit;
* Service Node candidate validation;
* epoch ID validation;
* Service Node ID validation;
* canonical BLAKE3 identifier validation.

This avoided duplicating:

* ID rules;
* candidate validation;
* economics validation;
* candidate-count bounds;
* BLAKE3 shape checks.

The helpers were not made broadly public.

---

# Canonical Enforcement Reward Review Constants

Added:

* `SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_SCHEMA`
* `SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_VERSION`

The canonical schema is:

`ron.rewarder.service-node-enforcement-review.v1`

These constants provide a stable identity for the Phase 19 reward-review wrapper.

---

# Enforcement-Aware Reward Review Entry Point

Added:

* `compute_service_node_reward_review_with_enforcement`

This function is the main Phase 19 reward-planning entry point.

It accepts:

* a `ServiceNodeRewardPlanInput`;
* canonical `ServiceNodeIdentityDescriptorV1` records;
* canonical `ServiceNodeEnforcementStatusV1` records;
* validated internal ROC reward-planning economics.

It returns:

* `ServiceNodeEnforcementRewardReview`

The function deliberately operates on registry-origin lifecycle and enforcement data rather than relying on candidate-supplied claims about eligibility or containment.

---

# Enforcement Review Processing Sequence

The reward review performs the following deterministic sequence:

1. validates canonical economics binding;
2. validates reward-plan identifiers and hashes;
3. validates candidate count and candidate fields;
4. sorts candidate input canonically;
5. hashes the complete original reward input;
6. validates and indexes descriptors;
7. requires an exact candidate-node and descriptor-node set match;
8. rejects `Candidate` lifecycle state;
9. identifies degraded, quarantined, and blocked nodes;
10. validates and indexes enforcement statuses;
11. requires exact enforcement-status coverage for contained nodes;
12. rejects enforcement statuses for nodes not in the candidate set;
13. requires status state and effective epoch to match the descriptor;
14. hashes the canonical descriptor and enforcement-status review material;
15. builds denial records for contained nodes;
16. removes contained candidates from planning input;
17. invokes the existing Phase 18 eligibility-aware planner for remaining nodes;
18. emits no nested reward plan if every candidate was denied;
19. derives a deterministic review ID;
20. validates the final review and non-authority posture.

This creates one explicit and auditable boundary between:

* accounting candidates;
* registry lifecycle truth;
* enforcement truth;
* reward planning.

---

# Contained Lifecycle States

The new reward-review path treats the following lifecycle states as contained:

* `Degraded`
* `Quarantined`
* `Blocked`

Candidates associated with these states are removed before reward allocation.

They cannot receive:

* normal reward allocation;
* probation-capped allocation;
* fallback allocation;
* dust allocation;
* an appeal-based allocation.

The rewarder does not reinterpret why a node is in a contained state.

It consumes the canonical state and enforcement status supplied by registry custody.

---

# Allowed Lifecycle States

Candidates may continue into the existing Phase 18 reward planner only when their descriptor state is:

* `Probation`
* `Eligible`

Probation candidates remain subject to the economics-configured probation reward cap already implemented in Phase 18.

Eligible candidates use the existing normal Service Node reward-planning rules.

The Phase 19 review therefore composes with, rather than replaces, the Phase 18 eligibility-aware planner.

---

# Candidate State Rejection

`Candidate` state is rejected before reward planning.

A newly registered node cannot gain access to the reward path merely because it appears in an accounting candidate list.

This preserves the Phase 18 invariant that candidate nodes have:

* no quorum authority;
* no normal reward eligibility;
* no automatic probation status;
* no trusted-node bypass.

---

# Exact Descriptor-Set Matching

The reward review requires the set of candidate Service Node IDs to exactly match the supplied descriptor set.

It rejects:

* missing descriptors;
* extra descriptors;
* duplicate descriptors;
* descriptors for unrelated nodes.

This prevents callers from:

* omitting a contained descriptor;
* supplying a clean descriptor for only honest-looking candidates;
* adding unrelated lifecycle evidence;
* bypassing state checks through incomplete registry input.

---

# Exact Enforcement-Status Matching

The reward review requires enforcement statuses for every contained node and only for contained nodes.

It rejects:

* a contained descriptor without a status;
* an extra status for an allowed node;
* a status for an unknown candidate;
* duplicate statuses;
* state mismatches;
* state-effective epoch mismatches.

This prevents a caller from presenting:

* `Blocked` in the descriptor but a stale `Quarantined` status;
* a current descriptor with an old enforcement epoch;
* an enforcement status for a different node;
* a missing reason or evidence record for a denied candidate.

---

# Service Node Reward Denial Record

Added `ServiceNodeRewardDenialV1`.

Each denial records:

* Service Node ID;
* canonical containment state;
* enforcement-status ID;
* violation kind;
* evidence root;
* effective epoch;
* visible appeal status;
* number of denied candidate rows.

This provides explicit evidence that a candidate was intentionally excluded rather than silently disappearing from a reward plan.

---

# Denial Record Validation

A denial record validates that:

* the Service Node ID is canonical;
* the state is `Degraded`, `Quarantined`, or `Blocked`;
* the status ID is non-empty;
* the effective epoch is nonzero;
* at least one candidate row was denied;
* the appeal shape is valid.

A denial record cannot claim reward planning permission.

It exposes helpers confirming that:

* reward planning is not permitted;
* economic mutation is not authorized.

---

# Multiple Candidate Rows Per Node

The denial builder counts all candidate rows associated with each contained Service Node.

This matters when one node has multiple evidence rows or content contributions in the same planning input.

The denial record contains:

* `candidate_rows`

This proves all of the contained node’s submitted reward material was excluded, not just the first row encountered.

---

# Honest Candidate Continuation

A major Phase 19 behavior added this session is that one bad candidate no longer has to abort the entire reward-planning batch.

Before this enforcement-aware wrapper, the existing Phase 18 planner correctly failed closed when it received contained descriptors.

That behavior was safe but meant a mixed set containing one blocked node and one eligible node could fail as a whole.

The new wrapper now:

1. validates the entire candidate and registry input;
2. records contained-node denials;
3. removes those candidates;
4. continues through the real Phase 18 planner for remaining valid nodes.

This allows honest Service Nodes to receive deterministic planned allocations even when another node in the same accounting candidate set is blocked or quarantined.

---

# Denial-Only Review

When all submitted candidates are contained, the function returns a successful:

* `ServiceNodeEnforcementRewardReview`

with:

* one or more denial records;
* `plan = None`.

This is intentionally not represented as:

* an empty fabricated reward plan;
* a zero-value payout plan;
* a failed computation;
* a wallet handoff;
* a receipt.

A denial-only result clearly states that the input was validly reviewed and all candidates were excluded.

---

# Enforcement Reward Review Artifact

Added `ServiceNodeEnforcementRewardReview`.

The artifact contains:

* schema;
* version;
* review ID;
* original reward-input hash;
* enforcement-review hash;
* sorted denial records;
* optional nested Phase 18 reward plan;
* planning-only posture;
* registry-attestation requirement;
* explicit non-authority flags.

---

# Review Non-Authority Fields

The review explicitly carries:

* `planning_only = true`
* `registry_attestation_required = true`
* `payout_authority = false`
* `payout_executed = false`
* `wallet_mutation = false`
* `ledger_mutation = false`
* `receipt_created = false`
* `balance_truth = false`

Validation rejects any artifact that crosses these boundaries.

This ensures the Phase 19 review cannot be mistaken for:

* a payout approval;
* wallet execution;
* a ledger receipt;
* account balance truth;
* an epoch finalization result.

---

# Nested Plan Exclusion Validation

When a nested reward plan exists, the wrapper validates it using the existing Phase 18 plan validation.

It additionally verifies that no denied Service Node appears in the nested allocation list.

If a denied node escapes into allocations, validation rejects the review as quarantined.

This protects against accidental filtering errors or later code changes that might reinsert contained candidates.

---

# Deterministic Input Ordering

Candidate input is sorted canonically by:

1. Service Node ID;
2. evidence class;
3. content ID.

Descriptors and enforcement statuses are also sorted by Service Node ID before indexing and hashing.

Denial records are emitted in sorted unique Service Node order.

This means equivalent inputs produce identical review results regardless of caller insertion order.

---

# Deterministic Reward Input Hash

The review computes a domain-separated BLAKE3 hash of the complete canonically ordered original reward input.

The hash binds:

* epoch ID;
* accounting snapshot CID;
* economics hash;
* policy hash;
* pool size;
* all original candidate rows.

Importantly, the hash is computed before contained candidates are removed.

This means the review identity commits to the full submitted candidate set, including denied material.

---

# Deterministic Enforcement Review Hash

The review computes a second domain-separated BLAKE3 hash over:

* canonical Service Node descriptors;
* canonical enforcement statuses.

This binds the reward result to the exact registry state and enforcement evidence used during review.

A change to:

* lifecycle state;
* state-effective epoch;
* violation reason;
* evidence root;
* appeal posture;
* enforcement status ID;

changes the enforcement review hash.

---

# Deterministic Review ID

The final review ID is a domain-separated BLAKE3 hash over:

* reward-input hash;
* enforcement-review hash;
* denial records;
* optional nested reward-plan ID.

This identity changes when any material reward or enforcement input changes.

It remains stable when semantically identical inputs are supplied in a different order.

---

# Canonical Economics Binding

The new review reuses `InternalRocRewardPlanningEconomics`.

It validates:

* economics configuration binding;
* economics configuration hash;
* reward pool nonzero posture;
* candidate-count limits;
* existing candidate economics constraints.

No Phase 19 reward amount, cap, split, or payout constant was hardcoded in the new module.

Probation and eligible allocations still derive from:

* `configs/roc-economics.toml`
* `configs/roc-economics.dev.toml`

through the existing validated economics loader.

---

# Candidate Validation Reuse

The enforcement-aware wrapper reuses the existing Service Node candidate validation.

Candidates must still satisfy all prior reward rules, including:

* canonical Service Node ID;
* canonical content ID;
* nonzero evidence count;
* nonzero eligible score;
* verified evidence;
* accepted accounting posture;
* passed policy gate;
* correct challenge posture;
* economics event limits.

Containment review is not a replacement for normal reward-material validation.

A malformed candidate fails before any denial or allocation review is produced.

---

# Reward-Recipient Binding Abuse

The new tests explicitly cover:

* `RewardRecipientBindingAbuse`

A Service Node quarantined for binding abuse produces:

* a denial record;
* no reward plan;
* no payout authority.

The enforcement status preserves the violation reason so the rewarder does not merely see a generic quarantine.

The rewarder does not attempt to:

* repair the binding;
* select a different payout recipient;
* redirect rewards to an operator account;
* pay a fallback address;
* retain the candidate under a reduced cap.

The node remains fully reward-denied.

---

# Pending Appeal Behavior

The new reward review preserves the appeal posture from registry enforcement status.

A pending appeal is visible in the denial record.

It does not:

* remove the denial;
* restore probation posture;
* restore eligible posture;
* create an allocation;
* authorize a payout.

This ensures an operator can see that an appeal exists while the economic path remains safely blocked.

---

# Accepted Appeal Posture

The protocol model allows accepted appeal status, but acceptance alone still does not restore lifecycle or reward eligibility.

The rewarder evaluates the canonical descriptor state supplied by registry custody.

If the descriptor remains quarantined or blocked, the node remains denied even when appeal metadata says `Accepted`.

A separate reviewed registry recovery transition is required before reward planning can resume.

---

# Blocked Node Behavior

Tests prove that a blocked node:

* receives no allocation;
* appears in `denied_service_nodes`;
* retains the canonical violation reason;
* cannot permit reward planning;
* cannot authorize economic mutation.

When the blocked node is the only candidate:

* `plan` is `None`;
* the review is denial-only.

When an eligible node is also present:

* the blocked node is denied;
* the eligible node continues into normal deterministic planning.

---

# Quarantined Node Behavior

Quarantined nodes receive the same reward exclusion posture as blocked nodes.

The rewarder does not treat quarantine as:

* a lower payout;
* a capped payout;
* a delayed payout;
* escrow;
* a pending reward.

The candidate is removed entirely from allocation planning.

---

# Degraded Node Behavior

Degraded nodes are also considered contained for Phase 19 reward review.

This preserves the policy decision that degraded nodes must not continue earning during unsafe or unresolved behavior.

The rewarder does not reinterpret degraded state as probation.

Recovery must occur through the canonical lifecycle state machine before rewards resume.

---

# Review Validation

`ServiceNodeEnforcementRewardReview::validate` verifies:

* schema;
* version;
* canonical review ID;
* canonical input hashes;
* sorted unique denials;
* each denial’s internal validity;
* nested reward-plan validity;
* denied-node exclusion from allocations;
* presence of either a plan or at least one denial;
* strict non-authority flags;
* deterministic review-ID recomputation.

This allows stored or transported review artifacts to be independently checked later.

---

# All-Candidates-Denied Helper

Added:

* `all_candidates_denied`

This returns true when:

* no nested reward plan exists.

It provides a simple and truthful way for future CLI, API, audit, or administrative surfaces to distinguish:

* mixed planning results;
* complete denial outcomes.

---

# Economic Mutation Helper

Added:

* `authorizes_economic_mutation`

This always returns false for the enforcement review.

The helper reinforces that the artifact is not:

* wallet authority;
* ledger authority;
* payout authority;
* mint authority;
* burn authority.

---

# Error Handling

The new module reuses `RewarderError`.

It produces fail-closed errors for:

* malformed identifiers;
* invalid economics binding;
* zero pool;
* empty candidate input;
* candidate-limit overflow;
* invalid candidate rows;
* invalid descriptors;
* candidate lifecycle state;
* duplicate descriptors;
* candidate/descriptor set mismatch;
* invalid enforcement statuses;
* status/descriptor mismatch;
* duplicate statuses;
* status set mismatch;
* hash encoding failures;
* denial ordering failures;
* nested allocation escape;
* authority-boundary violations.

The implementation does not silently drop malformed data.

Only valid contained candidates are converted into denial records.

---

# Tests Added

The focused Phase 19 rewarder suite contains five tests.

## `blocked_node_gets_no_allocation_while_eligible_node_plans`

Proves that a mixed input containing:

* one blocked node;
* one eligible node;

produces:

* one denial record for the blocked node;
* one normal allocation for the eligible node;
* no blocked-node allocation;
* no economic mutation authority.

This proves bad nodes do not suppress honest-node reward planning.

## `all_blocked_candidates_produce_denial_only_review`

Proves that when every candidate is blocked:

* no nested plan is created;
* a denial record is preserved;
* candidate-row count is retained;
* reward planning is not permitted.

## `reward_binding_abuse_denies_rewards_during_pending_appeal`

Proves that reward-recipient binding abuse:

* remains quarantined;
* remains reward-denied;
* preserves pending appeal visibility;
* does not create a reward plan.

## `missing_extra_or_mismatched_statuses_fail_closed`

Proves that reward review rejects:

* missing status for a contained descriptor;
* status state that does not match the descriptor.

This protects the registry-attestation boundary.

## `input_descriptor_and_status_order_do_not_change_review`

Proves that reordering:

* candidates;
* descriptors;
* statuses;

does not change the final enforcement reward review.

---

# Cross-Crate Tests Added Through `svc-registry`

The `svc-registry` Phase 19 integration suite was extended to call the real `svc-rewarder` enforcement-review function.

Although those tests reside in the registry crate, they directly exercise `svc-rewarder` behavior.

## Hash mismatch

Proves that a node quarantined through:

* `ron-proto`
* `ron-policy`
* `svc-registry`

produces a denial-only reward review in `svc-rewarder`.

## Invalid epoch signature

Proves that an invalid epoch signer:

* is quarantined;
* loses quorum posture;
* receives no reward plan.

## Denylist violation

Proves that a blocked denylist violator receives:

* no reward plan;
* a canonical denial record.

## Privacy leak with pending appeal

Proves that:

* privacy leak containment flows into reward denial;
* pending appeal status remains visible;
* the node remains excluded from rewards.

---

# Phase 18 Regression Protection

The new Phase 19 wrapper was tested alongside the existing Phase 18 probation and eligibility reward-plan suite.

Regression coverage confirmed that the new implementation did not break:

* probation reward caps;
* eligible-node normal caps;
* descriptor-set matching;
* descriptor-order determinism;
* lifecycle-history hashing;
* candidate-state rejection;
* degraded-state rejection in the lower-level planner;
* quarantined-state rejection in the lower-level planner;
* blocked-state rejection in the lower-level planner;
* economics-driven plan identity.

The Phase 18 probation-cap suite passed 6/6 tests during focused verification and again during the complete crate closeout.

---

# Existing Planner Behavior Preserved

The original:

* `compute_service_node_reward_plan_with_eligibility`

continues to fail closed when directly supplied:

* `Candidate`
* `Degraded`
* `Quarantined`
* `Blocked`

This behavior was not weakened.

The new Phase 19 function is an explicit higher-level wrapper that:

* validates containment status;
* creates denial records;
* filters contained candidates;
* passes only valid probationary and eligible nodes into the original planner.

This preserves backward safety while adding the required mixed-batch behavior.

---

# Full Crate Regression Coverage

The complete `svc-rewarder --all-targets` closeout passed.

Existing test families that remained green include:

* accounting epoch handoff;
* economics manifest binding;
* Service Node reward planning;
* probation reward caps;
* anti-farming event gates;
* config-driven planning;
* policy-gate interlock;
* payout intent boundaries;
* reward-plan boundaries;
* planning non-authority;
* wallet handoff boundaries;
* replay and deduplication;
* QuickChain authority scans;
* validator lifecycle boundaries;
* bond and dispute boundaries;
* anchor evidence boundaries;
* DA fallback boundaries;
* external posture boundaries;
* integration HTTP and wallet-client tests;
* unit and invariant suites.

The final output also confirmed:

* 5/5 Phase 19 enforcement reward-denial tests passed;
* 6/6 Phase 18 probation reward-cap tests passed;
* 41/41 general unit tests passed;
* the reward calculation benchmark completed successfully. 

---

# Verification Results

The focused Phase 19 reward-denial suite passed:

* 5 passed
* 0 failed

The Phase 18 probation-cap regression suite passed:

* 6 passed
* 0 failed

The crate passed:

* `cargo check -p svc-rewarder`;
* strict Clippy with `--all-targets --no-deps -- -D warnings`;
* `cargo test -p svc-rewarder --all-targets`;
* the final workspace compile check.

The full Phase 19 closeout completed with:

`PHASE19_FINAL_CLOSEOUT_STATUS=0`

---

# Resulting `svc-rewarder` Responsibilities

After this session, `svc-rewarder` now owns:

* enforcement-aware reward candidate review;
* exact descriptor-set validation;
* exact enforcement-status validation;
* contained-node exclusion;
* explicit deterministic denial records;
* denial-only review outcomes;
* honest-candidate continuation;
* pending-appeal visibility in reward denial;
* binding-abuse reward denial;
* canonical reward-input hashing;
* canonical enforcement-state hashing;
* deterministic review identities;
* nested allocation exclusion checks;
* continued use of economics-configured probation and eligible reward caps.

---

# Authority Boundary Preserved

The new implementation does not:

* determine whether a violation occurred;
* classify violation severity;
* mutate registry state;
* quarantine or block a node;
* resolve an appeal;
* rotate a reward binding;
* choose an alternate payout account;
* issue ROC;
* execute wallet payouts;
* mutate the ledger;
* create payout receipts;
* claim account balances;
* mint or burn ROC;
* mint or burn ROX;
* submit bridge or Solana transactions;
* claim settlement or finality.

The rewarder consumes canonical registry and policy outcomes and produces planning and denial artifacts only.

---

# Final Outcome

`svc-rewarder` now completes the economic side of Phase 19.

The full enforcement path is:

1. `ron-proto` defines the violation and enforcement contract.
2. `ron-policy` maps evidence to reviewed containment.
3. `svc-registry` applies containment to canonical lifecycle state.
4. `svc-rewarder` excludes contained nodes and records why.
5. Honest eligible or probationary nodes continue through the existing deterministic reward planner.
6. Pending appeals remain visible without restoring rewards.
7. No direct payout or ledger authority is introduced.

As a result, bad nodes can no longer remain in reward planning after canonical containment, and one contained node cannot prevent honest Service Nodes from receiving valid planned allocations.



### END NOTE - JULY 14 2026 - 01:05 CST


### BEGIN NOTE - JULY 15 2026 - 12:20 CST

Here are the drop-in changelog notes for the second crate, **`svc-rewarder`**.

## `svc-rewarder` — Phase 22 Session Changelog

### Added complete Phase 22 local reward-loop composition test

* Added `internal_roc_beta_phase22_local_reward_loop.rs`.
* Composed the existing economic-plane implementations into one deterministic local reward flow.
* Verified the full backend sequence:

```text
Service Node evidence
→ User Node verification evidence
→ accounting classification
→ canonical epoch snapshot
→ economics-bound reward handoff
→ capped Service Node reward plan
→ registry recipient resolution
→ eligible Service Node quorum
→ svc-wallet execution
→ durable ron-ledger receipt
→ independent User Node epoch replay
→ deterministic challenge on tampering
```

* Used the real shared types and validation behavior from the existing crates rather than creating a duplicate Phase 22 state machine.

### Added integration-only crate dependencies

Added development dependencies required by the cross-crate Phase 22 test:

* `micronode`
* `ron-kms`
* `ron-ledger`
* `svc-registry`
* `svc-wallet`

These dependencies are test-only and do not change `svc-rewarder` runtime authority.

### Bound reward planning to canonical economics

* Loaded the canonical `configs/roc-economics.toml` document through `ron-policy`.
* Loaded the same economics document through the `svc-rewarder` planning projection.
* Verified both crates calculate the same canonical economics configuration hash.
* Verified bridge and staking placeholders remain inert.
* Built the reward policy from economics-owned caps rather than test-local payout constants.
* Bound the reward plan, epoch transition, wallet operation, ledger receipt, and replay observation to the same economics identity.

### Integrated real accounting evidence

* Created canonical Service Node delivery evidence.
* Created canonical User Node verification evidence.
* Classified both evidence types through `ron-accounting`.
* Built a deterministic accounting epoch snapshot from the classified evidence.
* Verified the accounting snapshot produces a canonical BLAKE3 artifact identity.
* Preserved the accounting non-authority boundary:

  * no reward amount in raw evidence
  * no payout recipient in raw evidence
  * no wallet mutation
  * no ledger mutation
  * no confirmed ROC

### Verified deterministic accounting-to-rewarder handoff

* Converted the canonical accounting snapshot into reward-planning material using the existing accounting epoch handoff.
* Verified repeated handoff construction produces identical output.
* Verified Service Node evidence becomes a recipient-free reward candidate.
* Verified User Node verification remains a neutral point plan and does not receive an invented monetary rate.
* Confirmed the handoff remains:

  * planning-only
  * policy-gated
  * economics-bound
  * non-authoritative

### Verified deterministic capped reward planning

* Generated the Service Node reward plan using the real Phase 14 reward planner.
* Verified repeated planning with the same inputs produces an identical plan.
* Verified allocations retain the canonical `service_node_id`.
* Verified the allocation category comes from the accepted evidence class.
* Verified economics-owned epoch, category, account, content, and event caps apply.
* Verified the reward plan contains no caller-selected recipient.
* Preserved the rule that `svc-rewarder` creates planning material only.
* Confirmed the reward plan does not claim:

  * payout authority
  * payout execution
  * wallet mutation
  * ledger mutation
  * receipt creation
  * balance truth

### Integrated registry-derived reward recipients

* Added a canonical reward-binding registry fixture for three eligible Service Nodes.
* Resolved the rewarded Service Node to its registered reward-recipient account.
* Verified recipient resolution is derived from the trusted registry binding.
* Verified an arbitrary evidence-provided payout override is not used.
* Verified a different eligible Service Node may participate in approval of the beneficiary’s registry-bound payout.
* Verified a Service Node cannot approve issuance to its own bound reward recipient under the default posture.

### Verified self-issuance rejection

* Added an explicit assertion that self-issued payout authorization is rejected.
* Used `SelfIssuanceMode::RejectByDefault`.
* Confirmed the test-only fixture does not silently bypass the production self-pay rule.
* Preserved the rule that Service Nodes cannot directly reward their own operator or reward address.

### Added real Service Node quorum proof

* Generated three real Ed25519 Service Node keys through `ron-kms`.
* Built canonical eligibility records for three Service Nodes.
* Configured a two-of-three quorum threshold.
* Signed the epoch transition with two independent Service Node keys.
* Verified signatures through the production wallet quorum verifier.
* Bound signatures to:

  * chain identity
  * epoch identity
  * Service Node identity
  * key reference
  * transition hash
* Avoided fake signature-success strings or test-only finality claims.

### Verified single-node mint rejection

* Constructed a transition containing only one eligible Service Node signature.
* Submitted it through the real `svc-wallet` epoch-execution path.
* Verified execution fails before ledger mutation.
* Verified the recipient balance remains zero after the rejected attempt.
* Confirmed one Service Node cannot independently mint or finalize ROC.

### Integrated wallet execution

* Converted the quorum-authorized transition into registry-bound payout operations.
* Executed the transition through `svc-wallet`.
* Confirmed `svc-rewarder` itself does not directly mutate the ledger.
* Verified wallet execution requires:

  * canonical transition validation
  * matching economics identity
  * matching policy identity
  * matching registry root
  * matching reward-binding root
  * registry-resolved recipient
  * valid quorum signatures
  * sufficient quorum threshold

### Integrated durable ledger receipts

* Executed the approved operation through the local `ron-ledger` implementation.
* Verified a durable payout receipt is produced.
* Verified the receipt contains the registry-derived recipient account.
* Verified the receipt amount matches the deterministic capped reward-plan total.
* Verified the receipt retains the canonical economics configuration hash.
* Verified the ledger balance equals the issued reward amount.

### Verified idempotent retry and no double issuance

* Re-executed the exact same epoch transition.
* Verified the second execution returns the identical receipt set.
* Verified the recipient balance is not increased a second time.
* Verified exact replay does not create duplicate issuance.
* Confirmed the reward loop remains deterministic and idempotent.

### Integrated ledger replay

* Replayed the durable payout receipts through `ron-ledger`.
* Verified:

  * receipt count
  * total issued amount
  * recipient balance
  * total supply conservation
  * ledger sequence
  * ledger root
* Confirmed the sum of replayed balances equals the total issued amount.

### Integrated independent User Node epoch review

* Passed the completed epoch transition and replay observation into micronode’s independent economic auditor.
* Verified the User Node accepts the valid transition.
* Verified real Ed25519 quorum signatures are independently checked.
* Verified the User Node review performs no wallet or ledger mutation.
* Verified accepted review does not fabricate challenge submission.

### Added tamper detection and challenge proof

* Modified the replay observation’s reward-plan identity.
* Verified the User Node rejects the tampered replay.
* Built canonical invalid-epoch challenge evidence from the rejected review.
* Verified the challenge validates against the canonical schema.
* Verified the challenge is classified as `InvalidRewardPlan`.
* Preserved deterministic challenge identity and non-mutation behavior.

### Added confirmed-ROC projection export seam

* Extended the Phase 22G integration test with an optional environment-controlled projection export.
* Added `PHASE22_CONFIRMED_ROC_PROJECTION_PATH`.
* When a valid path is supplied, the completed reward loop exports a JSON projection derived from:

  * real wallet execution
  * durable ledger receipts
  * ledger replay
  * accepted User Node replay
* The projection includes:

  * epoch identity
  * recipient account
  * confirmed ROC minor units
  * receipt count
  * operation identities
  * ledger sequence
  * ledger root
  * transition hash
  * economics configuration hash
* The projection explicitly records:

  * wallet receipt confirmed
  * ledger replay confirmed
  * User Node replay accepted
  * pending evidence false
  * display-only true
  * client wallet mutation false
  * client ledger mutation false
  * client finality authority false

### Preserved confirmed-ROC truth boundaries

* The projection is emitted only after real receipt and replay validation.
* Pending micronode evidence is not used as confirmed ROC.
* Reward planning output is not used as confirmed ROC.
* Accounting snapshots are not used as confirmed ROC.
* Quorum material alone is not used as confirmed ROC.
* `ron-ledger` remains the durable economic truth source.
* CrabLink receives display material only and gains no mutation or finality authority.

### Added focused Phase 22 reward-loop runner

* Added `scripts/check-phase22-local-reward-loop.sh`.
* The runner executes:

  * the new complete Phase 22G integration test
  * accounting epoch snapshot regressions
  * accounting-to-rewarder handoff regressions
  * Service Node reward-plan regressions
  * reward-binding registry regressions
  * reward payout-guard regressions
  * wallet quorum-execution regressions
  * micronode epoch-replay regressions
* Added a conclusive Phase 22G success marker.

### Fixed strict Clippy findings

* Replaced cloned one-element evidence slices with `std::slice::from_ref`.
* Removed unnecessary `ServiceEvidenceAccountingInputV1` and `UserVerificationAccountingInputV1` clones.
* Preserved later ownership of both evidence values for evidence-root construction.
* Confirmed strict Clippy passes with `-D warnings` and `--no-deps`.

### Tests and verification

* Confirmed the new complete Phase 22G reward-loop test passes.
* Confirmed strict Clippy passes for the new integration test.
* Confirmed all 12 accounting snapshot regression tests pass.
* Confirmed all 6 accounting handoff tests pass.
* Confirmed all 8 Service Node reward-plan tests pass.
* Confirmed all 9 reward-binding registry tests pass.
* Confirmed all 6 reward payout-guard tests pass.
* Confirmed all 11 wallet quorum-execution tests pass.
* Confirmed all 18 User Node epoch-replay tests pass.
* Confirmed the complete Phase 22G regression runner passes.
* Confirmed `cargo check --workspace` passes after the Phase 22G changes.

### Current follow-up item

* The optional confirmed-ROC projection export is implemented.
* The remaining failure encountered at the end of the session was caused by the macOS temporary-file command, not by reward calculation or wallet/ledger behavior.
* `mktemp` failed to create the requested filename because the template included a suffix after `XXXXXX`.
* This left the projection path empty and caused the optional projection write to fail.
* The next session should repair temporary-file creation before rerunning the cross-repository Phase 22H projection test.
* The underlying Phase 22G reward loop remains green when no invalid projection path is supplied.

### Authority posture

* `svc-rewarder` remains deterministic reward planning infrastructure.
* `svc-rewarder` does not select arbitrary payout recipients.
* `svc-rewarder` does not directly mutate wallet or ledger state.
* `svc-rewarder` does not create balance truth.
* `svc-rewarder` does not independently finalize an epoch.
* `svc-rewarder` does not allow single-node minting.
* `svc-rewarder` does not permit default self-issuance.
* `svc-rewarder` remains bound to canonical economics, accounting, policy, registry, quorum, wallet, and ledger validation.

Next crate: **`crablink-tauri`**.


### END NOTE - JULY 15 2026 - 12:20 CST


### BEGIN NOTE - JULY 15 2026 - 18:25 CST

## svc-rewarder — Session Changelog

* Added the integrated Phase 22G local reward-loop coverage.
* Added confirmed-ROC projection export and validation for CrabLink.
* Added fail-closed dependency-outage behavior and readiness tests.
* Verified deterministic capped planning, registry-bound recipients, and idempotent wallet handoff.
* Confirmed no direct ledger mutation, fake payout, fake receipt, or finality authority.
* Fixed strict-Clippy issues and completed focused acceptance checks.


### END NOTE - JULY 15 2026 - 18:25 CST