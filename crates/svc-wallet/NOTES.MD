
````markdown
# svc-wallet NOTES.MD — Gold Plateau / Carryover Notes

Date: 2026-04-27  
Crate: `svc-wallet`  
Workspace path: `crates/svc-wallet`  
Project: RustyOnions / RON-CORE / WEB3 value plane  
Current status: **Gold test plateau reached**  
Completion estimate: **90–93% complete**  
Next WEB3 crate likely: `svc-rewarder`, unless we decide to first deepen paid-storage integration or durable wallet mode.

---

# 0. Executive Summary

`svc-wallet` has been taken from scaffold/stub state to a functioning, tested, runnable wallet service for the ROC internal economy.

The crate now provides:

- strict wallet DTOs
- deterministic error taxonomy
- integer-only `AmountMinor`
- deterministic receipt hashing
- idempotency store
- strict per-account nonce table
- capability/policy seams
- local `ron-ledger` adapter
- issue / transfer / burn / balance logic
- Axum HTTP service shell
- `/healthz`
- `/readyz`
- `/metrics`
- `/v1/balance`
- `/v1/issue`
- `/v1/transfer`
- `/v1/burn`
- `/v1/tx/{txid}`
- in-process dev state
- receipt book
- manual HTTP smoke success
- HTTP black-box tests
- invariant tests
- concurrency tests
- bench smoke targets
- CI/script/config name cleanup from `svc-wallet2` to `svc-wallet`

The final verified run passed:

```bash
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
cargo run -p svc-wallet
````

The final run also successfully launched:

```text
svc-wallet listening local_addr=127.0.0.1:8088
```

This crate is now strong enough to serve as the network-facing wallet boundary for the next WEB3 phase.

---

# 1. Strategic Role in WEB3 / ROC

The WEB3 blueprint defines ROC as an internal deterministic economy, not an external-chain token first. The correct value-plane sequence is:

```text
ron-ledger → svc-wallet → paid storage/pinning integration → svc-rewarder → visibility/dashboard → ROX seam later
```

`svc-wallet` is the **network boundary** and **mutation front-door** for the ROC economy.

It should not become the durable truth engine. That role belongs to `ron-ledger`.

It should not become the transient metering engine. That role belongs to `ron-accounting`.

It should not become the reward computation engine. That role belongs to `svc-rewarder`.

The correct boundaries are:

```text
ron-ledger:
  Durable truth, append-only commits, replay, roots, balances, receipts.

svc-wallet:
  Auth/policy boundary, DTO validation, nonce/idempotency, request shaping, receipt serving.

ron-accounting:
  Transient counters, usage windows, pricing inputs, metering signals.

svc-rewarder:
  Deterministic reward epochs, issuance intents, anti-gaming caps.

svc-storage / svc-gateway:
  First paid use-case enforcement path.

svc-admin / dashboard:
  Human visibility into supply, balances, receipts, usage, rewards.
```

The wallet now proves the key WEB3 invariant:

```text
Wallet is the sole mutation front-door with nonce + idempotency protection.
```

---

# 2. Major Architectural Decisions Locked During This Pass

## 2.1 Package identity fixed

The crate was incorrectly seen by Cargo as:

```text
svc-wallet2
```

This was fixed. Cargo now recognizes the package as:

```text
svc-wallet
```

We also cleaned supporting drift in `.cargo`, workflow files, scripts, and example config so future commands use `svc-wallet`, not `svc-wallet2`.

Important command going forward:

```bash
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
cargo run -p svc-wallet
```

## 2.2 Wallet stays service-first

The authoritative public contract is HTTP, not a large stable Rust embedding API.

The Rust modules are public enough for tests and internal crate composition right now, but the long-term API posture remains:

```text
HTTP is the stable surface.
Rust public surface should stay minimal/private where possible.
```

## 2.3 Ledger primacy preserved

`svc-wallet` does not invent a second balance truth.

Balances are read through the ledger adapter. Mutations commit through the local ledger client. Tests prove rejected commits do not create partial credits.

## 2.4 Amnesia-first dev posture

The current working dev mode is in-memory and amnesia-safe.

This is correct for local/Micronode-style development. Durable wallet/ledger persistence is a later explicit design decision, not hidden default behavior.

## 2.5 No floats

All money-like values are integer minor units.

DTOs use string-encoded amount fields at the JSON boundary so large integer precision is not lost.

## 2.6 Strict nonce model

Debit-side operations use strict per-account nonce sequencing.

Current v1 model:

```text
nonce starts at 1
same account + same nonce can commit at most once
nonce replay rejects deterministically
rollback can reopen a reservation only if the commit path fails before finalization
```

## 2.7 Idempotency model

Write endpoints require/rely on idempotency.

Rules:

```text
same idempotency key + same canonical request => replay same receipt
same idempotency key + different canonical request => 409 IDEMPOTENCY_CONFLICT
```

HTTP black-box tests prove byte-identical idempotent replay for issue.

## 2.8 Receipt hashes are deterministic

Receipts are finalized with a BLAKE3-style `b3:` hash.

Tests prove:

```text
same receipt vector => same receipt hash
amount mutation => different receipt hash
txid/request fingerprint generation is deterministic
```

## 2.9 Current ledger amount ceiling

Wallet DTOs model `AmountMinor` as `u128`.

The current `ron-ledger` adapter path rejects values above `u64::MAX`.

This is an intentional safety bridge until `ron-ledger` itself is widened to full `u128` internally.

Do not remove this guard unless `ron-ledger` has been upgraded and tested.

---

# 3. Implementation Phases Completed

## Phase 1 — Core/domain implementation

Phase 1 filled the foundation:

```text
src/config.rs
src/dto/errors.rs
src/dto/mod.rs
src/dto/requests.rs
src/dto/responses.rs
src/errors.rs
src/idem/mod.rs
src/idem/store.rs
src/seq/mod.rs
src/seq/nonce.rs
src/util/blake3_receipt.rs
src/util/headers.rs
src/util/parsing.rs
src/auth/caps.rs
src/auth/mod.rs
src/policy/enforce.rs
src/policy/mod.rs
src/accounting/client.rs
src/accounting/mod.rs
src/cache/balance_cache.rs
src/cache/invalidator.rs
src/cache/mod.rs
src/ledger/client.rs
src/ledger/mod.rs
src/ledger/types.rs
src/lib.rs
Cargo.toml
```

Core features added:

* config defaults and validation
* hardening constants
* amount parsing/serialization
* string-encoded JSON amount fields
* strict DTO parsing
* error taxonomy
* idempotency key store
* nonce reservation/rollback/commit
* capability claims/scopes
* local policy enforcement
* receipt hashing/fingerprinting
* txid generation
* local in-memory `ron-ledger` adapter
* issue/transfer/burn/balance methods
* unit tests for core modules

Phase 1 result:

```text
16 lib tests passed
core domain layer compiled and worked
```

## Phase 2 — HTTP service shell

Phase 2 filled service-level files:

```text
src/routes/mod.rs
src/routes/health.rs
src/routes/metrics.rs
src/routes/v1/mod.rs
src/routes/v1/balance.rs
src/routes/v1/issue.rs
src/routes/v1/transfer.rs
src/routes/v1/burn.rs
src/routes/v1/receipt.rs
src/readiness.rs
src/metrics.rs
src/main.rs
src/supervisor.rs
src/middleware/*
```

Service features added:

* Axum router
* `WalletState::dev`
* `/healthz`
* `/readyz`
* `/metrics`
* balance endpoint
* issue endpoint
* transfer endpoint
* burn endpoint
* receipt lookup endpoint
* receipt book
* route-level auth requirement
* metrics counters/gauges
* basic readiness state
* runnable `svc-wallet` binary

Phase 2 result:

```text
28 lib tests passed
service booted on 127.0.0.1:8088
manual curl flow succeeded
```

Manual flow proved:

```text
GET /healthz
GET /readyz
GET /metrics
GET /v1/balance?account=acct_a&asset=roc
POST /v1/issue
POST /v1/transfer
GET /v1/balance?account=acct_a&asset=roc
GET /v1/balance?account=acct_b&asset=roc
```

Manual result:

```text
acct_a started at 0
issue +100 to acct_a
transfer 40 from acct_a to acct_b
acct_a ended at 60
acct_b ended at 40
```

## Phase 3 — Invariant integration tests

Phase 3 converted placeholder tests into real tests:

```text
tests/api_contract.rs
tests/harness.rs
tests/i_1_doublespend.rs
tests/i_2_nonnegativity.rs
tests/i_3_conservation.rs
tests/i_4_caps_policy.rs
tests/i_5_dto_hygiene.rs
tests/i_7_ledger_primacy.rs
tests/i_8_observability.rs
tests/i_9_amnesia.rs
tests/i_11_transport_bounds.rs
tests/i_12_overflow_ceilings.rs
tests/readiness_and_shedding.rs
tests/vectors_receipt_hash.rs
```

Phase 3 proved:

* API contract shape
* amount string encoding
* unknown fields reject
* no doublespend via nonce
* idempotency replay/conflict
* non-negativity
* transfer conservation
* burn supply reduction
* missing cap scope rejects
* asset/account policy caveats reject
* DTO hygiene
* no ambient auth fields in JSON
* wrong asset rejects
* control chars in memo reject
* ledger primacy
* rejected commit does not create partial state
* stable error codes
* retryability semantics
* amnesia defaults
* RAM-only idempotency TTL purge
* fresh in-memory ledger starts empty
* 1 MiB body cap
* 10x decompression cap
* 5s timeout
* 512 inflight default
* zero amount rejection
* max amount ceiling rejection
* current `u64` ledger adapter ceiling
* receipt hash determinism

Phase 3 result:

```text
66 real tests passed after this phase
```

## Phase 4 — HTTP black-box tests

Phase 4 added:

```text
tests/http_blackbox.rs
```

Initial HTTP black-box coverage proved:

* health endpoint
* readiness endpoint
* metrics endpoint
* issue through actual router
* transfer through actual router
* balance through actual router
* receipt lookup through actual router
* idempotent replay returns same receipt bytes
* same idempotency key with different body rejects
* nonce replay rejects through HTTP
* missing authorization rejects
* unknown receipt returns 404

Phase 4 result:

```text
7 HTTP black-box tests passed
```

## Phase 5 — Final gold polish

Phase 5 added/fixed:

```text
.cargo/config.toml
.github/workflows/ci.yml
.github/workflows/loom.yml
.github/workflows/render-mermaid.yml
configs/svc-wallet.example.toml
configs/policies.sample.json
scripts/dev-run.sh
scripts/reload-policy.sh
scripts/chaos-ledger-stall.sh
tests/loom_invariants.rs
tests/http_blackbox.rs
```

Phase 5 closed:

* remaining `svc-wallet2` drift in local aliases/workflows/scripts/config
* empty `loom_invariants.rs`
* burn route HTTP coverage
* metrics-after-success/replay/reject coverage
* metrics issue/burn op counters
* metrics idempotency replay counter
* metrics reject counter for `IDEMPOTENCY_CONFLICT`
* concurrency tests for nonce and idempotency gate behavior

Phase 5 result:

```text
79 real tests passed
2 bench smoke runs passed
svc-wallet booted successfully
```

---

# 4. Final Verified Test Suite

Final count:

```text
28 lib tests
4 API contract tests
9 HTTP black-box tests
3 transport bounds tests
3 overflow ceiling tests
2 doublespend tests
2 nonnegativity tests
2 conservation tests
4 caps/policy tests
4 DTO hygiene tests
2 ledger primacy tests
3 observability tests
3 amnesia tests
4 concurrency/loom-style tests
3 readiness/shedding tests
3 receipt hash vector tests
2 bench smoke runs
```

Total:

```text
79 real passing tests
2 bench smoke runs
```

Final commands green:

```bash
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
cargo run -p svc-wallet
```

Final service launch:

```text
svc-wallet listening local_addr=127.0.0.1:8088
```

---

# 5. Endpoint Contract Currently Working

## Public operational endpoints

```text
GET /healthz
GET /readyz
GET /metrics
```

## Wallet API endpoints

```text
GET  /v1/balance?account=<account>&asset=roc
POST /v1/issue
POST /v1/transfer
POST /v1/burn
GET  /v1/tx/{txid}
```

## Current auth posture

The HTTP path expects an `Authorization: Bearer ...` header for `/v1/*`.

In dev mode, tests use:

```text
Authorization: Bearer dev
```

This is a placeholder/dev verifier path.

Production still needs real integration with:

```text
ron-auth
svc-passport
ron-policy
svc-registry
```

## Current idempotency posture

Write routes use:

```text
Idempotency-Key: <key>
```

Current tested behavior:

```text
same key + same body => 200 same receipt bytes
same key + different body => 409 IDEMPOTENCY_CONFLICT
```

## Current nonce posture

Debit operations require nonce.

Currently tested:

```text
first transfer nonce 1 succeeds
second transfer nonce 1 rejects with NONCE_CONFLICT
burn uses nonce and reduces balance
```

---

# 6. Metrics Surface Currently Proven

Metrics endpoint renders wallet-specific series.

Important current series:

```text
wallet_requests_total
wallet_successes_total
wallet_idempotency_replays_total
wallet_inflight
wallet_ready
wallet_rejects_total{reason="..."}
wallet_ops_total{op="issue"}
wallet_ops_total{op="transfer"}
wallet_ops_total{op="burn"}
```

HTTP black-box tests now prove metrics reflect:

* successful issue
* idempotent replay
* idempotency conflict reject
* burn operation
* readiness gauge

Current metrics test specifically expects:

```text
wallet_idempotency_replays_total 1
wallet_ops_total{op="issue"} 1
wallet_ops_total{op="burn"} 1
wallet_rejects_total{reason="IDEMPOTENCY_CONFLICT"} 1
wallet_ready 1
```

Future metrics work should avoid renaming these lightly. Dashboards and admin views can build on them.

---

# 7. Invariants Now Covered

## I-1 No doublespends

Covered by:

```text
tests/i_1_doublespend.rs
tests/loom_invariants.rs
tests/http_blackbox.rs
```

Proves:

* nonce reservation accepts exactly once
* same nonce cannot commit twice
* nonce rollback reopens only failed path
* idempotency replay works
* idempotency conflict rejects
* HTTP nonce replay rejects

## I-2 Non-negativity

Covered by:

```text
tests/i_2_nonnegativity.rs
```

Proves:

* transfer from empty account rejects
* burn more than balance rejects
* receiver is not credited on failed transfer
* balance is unchanged on failed burn

## I-3 Conservation

Covered by:

```text
tests/i_3_conservation.rs
```

Proves:

* transfer preserves participant sum
* issue creates explicit supply
* burn destroys explicit supply
* transfer receipt spans debit + credit sequence entries

## I-4 Capability/policy gating

Covered by:

```text
tests/i_4_caps_policy.rs
tests/http_blackbox.rs
```

Proves:

* missing scope rejects
* empty bearer token rejects
* account caveat denies unlisted counterparty
* asset caveat applies
* amount over policy ceiling rejects
* missing HTTP auth rejects

Remaining:

* replace dev verifier with real `ron-auth`/`svc-passport` verifier
* enforce real TTL/caveat token semantics at route layer

## I-5 DTO hygiene

Covered by:

```text
tests/i_5_dto_hygiene.rs
tests/api_contract.rs
```

Proves:

* unknown fields reject
* ambient `authorization` field in JSON rejects
* floats reject
* zero amount rejects
* wrong asset rejects
* control chars in memo reject
* amount serializes as string

## I-7 Ledger primacy

Covered by:

```text
tests/i_7_ledger_primacy.rs
```

Proves:

* balance reads follow successful commits only
* failed transfer does not create partial credit
* wallet does not maintain alternate balance truth

## I-8 Observability

Covered by:

```text
tests/i_8_observability.rs
tests/http_blackbox.rs
```

Proves:

* stable machine error codes
* stable retryability classes
* dashboard-safe uppercase labels
* metrics endpoint live
* metrics reflect success/replay/reject/op counts

## I-9 Amnesia

Covered by:

```text
tests/i_9_amnesia.rs
```

Proves:

* default config is amnesia-first
* idempotency state expires from RAM
* fresh in-memory ledger starts empty

Remaining:

* explicit durable mode later
* explicit zero-disk integration test if/when persistence option exists

## I-11 Transport/hardening bounds

Covered by:

```text
tests/i_11_transport_bounds.rs
src/middleware/*
```

Proves:

* body cap defaults to 1 MiB
* decompression ratio defaults to 10x
* request timeout defaults to 5s
* max inflight defaults to 512
* invalid caps reject

## I-12 Overflow ceilings

Covered by:

```text
tests/i_12_overflow_ceilings.rs
```

Proves:

* zero amount rejects
* amount above config ceiling rejects
* values above `u64::MAX` reject at current ledger adapter boundary

---

# 8. Current Source Layout

Current important source modules:

```text
src/accounting/client.rs
src/accounting/mod.rs
src/auth/caps.rs
src/auth/mod.rs
src/cache/balance_cache.rs
src/cache/invalidator.rs
src/cache/mod.rs
src/config.rs
src/dto/errors.rs
src/dto/mod.rs
src/dto/requests.rs
src/dto/responses.rs
src/errors.rs
src/idem/mod.rs
src/idem/store.rs
src/ledger/client.rs
src/ledger/mod.rs
src/ledger/types.rs
src/lib.rs
src/main.rs
src/metrics.rs
src/middleware/decompress_cap.rs
src/middleware/limits.rs
src/middleware/mod.rs
src/middleware/rate_limit.rs
src/middleware/request_id.rs
src/middleware/shedder.rs
src/middleware/timeouts.rs
src/middleware/tracing_log.rs
src/policy/enforce.rs
src/policy/mod.rs
src/readiness.rs
src/routes/health.rs
src/routes/metrics.rs
src/routes/mod.rs
src/routes/v1/balance.rs
src/routes/v1/burn.rs
src/routes/v1/issue.rs
src/routes/v1/mod.rs
src/routes/v1/receipt.rs
src/routes/v1/transfer.rs
src/seq/mod.rs
src/seq/nonce.rs
src/supervisor.rs
src/util/blake3_receipt.rs
src/util/headers.rs
src/util/mod.rs
src/util/parsing.rs
```

Current important tests:

```text
tests/api_contract.rs
tests/harness.rs
tests/http_blackbox.rs
tests/i_1_doublespend.rs
tests/i_2_nonnegativity.rs
tests/i_3_conservation.rs
tests/i_4_caps_policy.rs
tests/i_5_dto_hygiene.rs
tests/i_7_ledger_primacy.rs
tests/i_8_observability.rs
tests/i_9_amnesia.rs
tests/i_11_transport_bounds.rs
tests/i_12_overflow_ceilings.rs
tests/loom_invariants.rs
tests/readiness_and_shedding.rs
tests/vectors_receipt_hash.rs
```

Current bench smoke files:

```text
benches/balance_read.rs
benches/transfer_commit.rs
```

Current ops/config/workflow files touched:

```text
.cargo/config.toml
.github/workflows/ci.yml
.github/workflows/loom.yml
.github/workflows/render-mermaid.yml
configs/svc-wallet.example.toml
configs/policies.sample.json
scripts/dev-run.sh
scripts/reload-policy.sh
scripts/chaos-ledger-stall.sh
```

---

# 9. Important Runtime Defaults

Current defaults that tests lock:

```text
asset = "roc"
amnesia = true
max_body_bytes = 1_048_576
max_decomp_ratio = 10
req_timeout_ms = 5_000
max_inflight = 512
default_staleness_ms = 250
idempotency_ttl_secs = 24h
max_amount_per_op = 10^20 minor units
NONCE_START = 1
```

Do not change these defaults without updating:

```text
src/config.rs
configs/svc-wallet.example.toml
tests/i_11_transport_bounds.rs
tests/i_12_overflow_ceilings.rs
docs/API.MD
docs/CONFIG.MD
docs/TESTS.MD
```

---

# 10. Current Error Taxonomy

Important wallet error codes tested:

```text
BAD_REQUEST
UNAUTHORIZED
FORBIDDEN
LIMITS_EXCEEDED
INSUFFICIENT_FUNDS
NONCE_CONFLICT
IDEMPOTENCY_CONFLICT
BUSY
RETRY_LATER
UPSTREAM_UNAVAILABLE
NOT_FOUND
```

HTTP status mapping currently tested includes:

```text
UNAUTHORIZED => 401
IDEMPOTENCY_CONFLICT => 409
NONCE_CONFLICT => 409
INSUFFICIENT_FUNDS => 409
NOT_FOUND => 404
BUSY => 429
RETRY_LATER => 503
```

Retryability currently tested:

```text
BUSY => retryable
RETRY_LATER => retryable
UPSTREAM_UNAVAILABLE => retryable

BAD_REQUEST => not retryable
FORBIDDEN => not retryable
INSUFFICIENT_FUNDS => not retryable
```

Error labels are expected to be dashboard-safe uppercase strings with underscores.

---

# 11. Manual Smoke Commands

Run the service:

```bash
cargo run -p svc-wallet
```

Check health/readiness/metrics:

```bash
curl -s http://127.0.0.1:8088/healthz
curl -s http://127.0.0.1:8088/readyz
curl -s http://127.0.0.1:8088/metrics
```

Balance before issue:

```bash
curl -s 'http://127.0.0.1:8088/v1/balance?account=acct_a&asset=roc' \
  -H 'Authorization: Bearer dev'
```

Issue:

```bash
curl -s -X POST http://127.0.0.1:8088/v1/issue \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: idem_issue_1' \
  -H 'Content-Type: application/json' \
  -d '{"to":"acct_a","asset":"roc","amount_minor":"100","memo":null}'
```

Transfer:

```bash
curl -s -X POST http://127.0.0.1:8088/v1/transfer \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: idem_transfer_1' \
  -H 'Content-Type: application/json' \
  -d '{"from":"acct_a","to":"acct_b","asset":"roc","amount_minor":"40","nonce":1,"memo":null}'
```

Burn:

```bash
curl -s -X POST http://127.0.0.1:8088/v1/burn \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: idem_burn_1' \
  -H 'Content-Type: application/json' \
  -d '{"from":"acct_a","asset":"roc","amount_minor":"10","nonce":2,"memo":null}'
```

Read final balances:

```bash
curl -s 'http://127.0.0.1:8088/v1/balance?account=acct_a&asset=roc' \
  -H 'Authorization: Bearer dev'

curl -s 'http://127.0.0.1:8088/v1/balance?account=acct_b&asset=roc' \
  -H 'Authorization: Bearer dev'
```

Receipt lookup:

```bash
curl -s 'http://127.0.0.1:8088/v1/tx/<txid>' \
  -H 'Authorization: Bearer dev'
```

---

# 12. What Is Done

## Core correctness

Done:

* amount type
* strict DTOs
* validation
* stable errors
* receipt hashing
* txid generation
* request fingerprinting
* idempotency store
* nonce table
* rollback behavior
* local policy enforcement
* capability claims and scopes
* local ledger adapter
* issue
* transfer
* burn
* balance
* receipt generation
* receipt lookup via service state

## HTTP surface

Done:

* router
* state
* health
* readiness
* metrics
* balance
* issue
* transfer
* burn
* tx receipt lookup
* route-level auth requirement
* idempotency behavior
* nonce conflict behavior
* not found behavior

## Observability

Done:

* metrics rendering
* success counters
* request counters
* replay counter
* reject counters
* op counters
* ready gauge
* test coverage for metrics after success/replay/reject

## Resilience / hardening

Done:

* config validation for major bounds
* body cap helper
* decompression cap helper
* timeout helper
* rate-limit decision helper
* shedder helper
* readiness state
* BUSY/RETRY_LATER semantics

## Tests

Done:

* unit tests
* integration/invariant tests
* HTTP black-box tests
* concurrency tests
* bench smoke runs

## Ops polish

Done:

* package identity corrected
* workflow/package drift corrected
* scripts renamed/cleaned around `svc-wallet`
* example config updated
* sample policy updated

---

# 13. What Remains for svc-wallet

This crate is strong, but not fully “production final.” Remaining work is integration/productization, not basic correctness.

## 13.1 Real capability verifier

Current dev mode uses a static/dev verifier path.

Need:

```text
ron-auth integration
svc-passport integration
real macaroon parsing/verification
scope enforcement from real token caveats
TTL checks
account caveat checks
asset caveat checks
redacted logging
```

Tests to add later:

```text
valid read token can call balance
transfer token cannot issue
expired token rejects
wrong asset caveat rejects
wrong account caveat rejects
Authorization header never appears in logs
```

## 13.2 Durable wallet/ledger mode

Current mode is in-memory and amnesia-safe.

Need explicit decision:

```text
Keep wallet local state RAM-only always?
Or add durable idempotency/receipt book for non-amnesia profiles?
Should durable truth remain only in ron-ledger?
```

Recommended direction:

```text
ron-ledger owns durable balances/receipts.
svc-wallet may keep only derivative caches and idempotency helper state.
For non-amnesia, idempotency records may need durable storage if HTTP retry safety must survive process restart.
```

Open question:

```text
Should idempotency be wallet-local durable state, ledger-native durable state, or both?
```

Best long-term answer:

```text
Ledger should persist idempotency identity with committed transactions.
Wallet can cache idempotency decisions but should be able to recover replay behavior from ledger.
```

## 13.3 Hold / capture / release

Not implemented yet.

This is the biggest remaining functional piece before paid storage/pinning.

Needed endpoints likely:

```text
POST /v1/hold
POST /v1/capture
POST /v1/release
GET  /v1/hold/{hold_id}
```

Required invariants:

```text
hold max estimated amount before paid operation
capture actual cost after completion
release remainder
hold terminal state required
idempotent hold/capture/release
no double capture
no release after full capture
no capture after full release
partial capture then release remainder allowed
escrow account semantics deterministic
```

Canonical account shape recommended:

```text
t:<tenant>/u:<user>/w
t:<tenant>/treasury
t:<tenant>/escrow/<hold_id>
t:<tenant>/s:<service>/<bucket>
```

This must be coordinated with `ron-ledger`, because durable hold state belongs in the ledger truth model.

## 13.4 Real accounting integration

Current `accounting` path is a noop/dev seam.

Need:

```text
ron-accounting client adapter
metering event emission
bytes stored / bytes served / pin duration / request counts
pricing snapshot input
correlation IDs
error metrics for accounting unavailable
```

Important: `ron-accounting` must remain transient counters, not truth.

## 13.5 Rewarder integration

Future `svc-rewarder` should not mutate ledger directly.

Correct path:

```text
svc-rewarder computes deterministic issuance intents
svc-rewarder submits issue requests through svc-wallet or a narrow wallet/ledger commit seam
svc-wallet enforces privileged issue scope/policy
ron-ledger records durable issuance truth
```

Need to decide whether rewarder uses HTTP wallet API or internal crate/client trait.

Safer v1:

```text
rewarder -> svc-wallet HTTP/API with privileged rewarder capability
```

## 13.6 OpenAPI sync

Docs mention OpenAPI as desired source/contract.

Need:

```text
docs/openapi/svc-wallet.yaml
OpenAPI validation CI
handler/spec parity tests
example curl tests
stable schema for SDK/dashboard
```

Current API is implemented, but OpenAPI file generation/sync is not done.

## 13.7 Config loader

Current `WalletConfig` exists and sample config exists.

Need:

```text
load from TOML/env
validate config at startup
support bind address override
support amnesia flag
support policy path
support ledger/accounting endpoints
support max amount / inflight / timeout knobs
```

Current service uses dev defaults.

## 13.8 `/version`

Docs mention version-style operational surfaces in broader project. `svc-wallet` currently has health/ready/metrics. Consider adding:

```text
GET /version
```

Return:

```json
{
  "service": "svc-wallet",
  "version": "0.1.0",
  "profile": "dev",
  "amnesia": true
}
```

## 13.9 Fuzz targets

Fuzz target scaffolds exist:

```text
fuzz/fuzz_targets/dto_spend_parser.rs
fuzz/fuzz_targets/sequence_machine.rs
fuzz/fuzz_targets/spend_request_roundtrip.rs
```

Need to verify they compile and run.

Future commands:

```bash
cargo fuzz run dto_spend_parser
cargo fuzz run sequence_machine
cargo fuzz run spend_request_roundtrip
```

## 13.10 Load tests

Load scaffolds exist:

```text
testing/load/k6-transfer.js
testing/load/vegeta-balance.sh
```

Need to verify/update them against the actual current `/v1/*` API.

## 13.11 Public API tightening

Currently modules are public for internal/test convenience.

Docs say Rust surface should be minimal because HTTP is the contract.

Later tightening pass:

```text
make modules pub(crate) where feasible
keep only intentional public types
run cargo public-api
prevent accidental external Rust API drift
```

Do this carefully after downstream crates stop relying on internals.

---

# 14. Known Risks / Design Cautions

## 14.1 Do not let wallet become ledger

Danger:

```text
svc-wallet starts storing durable balances, receipts, and transaction history independently.
```

Avoid this.

Correct rule:

```text
wallet validates and fronts mutations
ledger is durable truth
```

## 14.2 Do not let accounting become ledger

`ron-accounting` is transient metering.

Do not use accounting counters as spendable balances.

## 14.3 Do not skip holds for paid storage

Direct debit is acceptable only for a tiny v0 demo if clearly documented.

Real paid storage/pinning should use:

```text
hold -> capture -> release
```

Without holds, long-running paid operations can fail awkwardly if cost estimates change or operation completion differs from admission.

## 14.4 Do not introduce external chain logic yet

ROX/Solana/external settlement is deferred.

The current goal is internal ROC utility and closed-loop proof.

## 14.5 Do not weaken idempotency

Idempotency replay must stay deterministic.

Danger signs:

```text
receipt timestamp differs on replay
receipt hash differs on replay
same key + different body accidentally accepted
idempotency cache expiry creates duplicate commit while ledger has prior tx
```

Long-term fix: durable idempotency anchored in ledger.

## 14.6 Do not weaken nonce rules too early

Strict nonce is easier to prove.

Avoid nonce windows until we have stronger tests and a real need.

## 14.7 Be careful with account grammar

Current test accounts like `acct_a` and `acct_b` are dev-simple.

Production should move toward canonical account strings:

```text
t:<tenant>/u:<user>/w
t:<tenant>/treasury
t:<tenant>/escrow/<hold_id>
```

Need a migration or acceptance decision later.

---

# 15. Recommended Next Crate Work

The next WEB3 crate depends on whether the priority is “reward loop” or “paid use case.”

## Option A — Move to `svc-rewarder` next

This is likely the natural next crate.

Reason:

```text
ron-ledger is green
svc-wallet is gold plateau
rewarder can now produce issuance intents and use wallet as mutation front-door
```

What to build in `svc-rewarder`:

```text
deterministic epoch config
input snapshot DTOs
accounting snapshot adapter
reward formula
anti-gaming caps
idempotency for reward epochs
issuance intent generation
wallet client adapter
dry-run endpoint
commit endpoint
metrics
readiness
tests for deterministic replay
```

Critical rule:

```text
svc-rewarder should not mutate ron-ledger directly in normal operation.
It should commit through svc-wallet or a narrow approved wallet/ledger interface.
```

Suggested rewarder first proof:

```text
given deterministic accounting snapshot
compute provider reward
submit issue to svc-wallet
provider balance increases
same epoch replay does not double issue
metrics show reward epoch complete
```

## Option B — Integrate paid storage/pinning next

This may be better if the goal is to prove “real in-network use case” before rewards.

Needed crates:

```text
svc-wallet
svc-storage
svc-gateway
ron-accounting
ron-policy
```

Required flow:

```text
user has 0 ROC => paid write rejects
issue ROC to user => paid write admitted
wallet hold created for estimated cost
storage operation completes
actual cost captured
remainder released
accounting records usage
metrics show paid operation
```

This option probably requires hold/capture/release first.

## Option C — Deepen `ron-accounting` next

If accounting is still too stubbed to feed rewarder, build it next.

Need:

```text
metering event schema
time windows
rollups
export snapshots
idempotent ingest
WAL/replay if already designed
rewarder input compatibility
```

Best order if pursuing full WEB3 closed loop:

```text
svc-wallet gold plateau
ron-accounting snapshot readiness
svc-rewarder deterministic issuance
svc-storage/gateway paid path
dashboard/admin visibility
```

---

# 16. Suggested Next Session Prompt

Use this in the next session:

```text
We finished svc-wallet to a gold test plateau. It now has 79 real passing tests, 2 bench smoke runs, HTTP black-box coverage for issue/transfer/burn/balance/receipt lookup, nonce/idempotency/concurrency tests, metrics tests, and it boots on 127.0.0.1:8088. The remaining svc-wallet work is production integration: real ron-auth/svc-passport capabilities, durable idempotency decision, config loader, hold/capture/release, OpenAPI sync, and real accounting/rewarder integration.

Next we are moving to the next WEB3 crate. Use WEB3.MD as north star. Remember: ROC stays internal; ROX/Solana is deferred. Wallet is the mutation front-door. ron-ledger is truth. ron-accounting is transient counters. svc-rewarder should compute deterministic issuance intents and commit through wallet, not mutate ledger directly. Please review the codebundle/all-docs for the next crate and continue from this state.
```

---

# 17. Suggested Commit Summary

Possible commit title:

```text
svc-wallet: implement ROC wallet service core, HTTP API, invariants, and gold tests
```

Possible commit body:

```text
- Fix package identity to svc-wallet and remove svc-wallet2 drift
- Add wallet config, DTOs, error taxonomy, nonce/idempotency gates
- Add deterministic receipt hashing and tx/request fingerprints
- Add local ron-ledger adapter for issue, transfer, burn, and balance
- Add Axum routes for /healthz, /readyz, /metrics, and /v1 wallet API
- Add receipt lookup and in-process dev wallet state
- Add metrics counters/gauges for requests, successes, operations, replays, rejects, readiness, inflight
- Add invariant tests for doublespend, non-negativity, conservation, caps/policy, DTO hygiene, ledger primacy, observability, amnesia, bounds, overflow ceilings, receipt vectors
- Add HTTP black-box tests for issue, transfer, burn, balance, receipt lookup, auth, idempotency, nonce conflict, metrics
- Add concurrency tests for nonce and idempotency behavior
- Add dev scripts, example config, sample policy, and CI workflow cleanup
- Confirm cargo fmt, clippy -D warnings, cargo test --all-targets, benches, and cargo run all pass
```

---

# 18. Final Status

`svc-wallet` is now a strong internal ROC wallet service foundation.

It is safe to move on.

Do not keep adding random features to `svc-wallet` before the next crate unless the feature directly supports one of these remaining integration goals:

```text
real caps
durable idempotency/replay
hold/capture/release
accounting integration
rewarder integration
OpenAPI sync
paid storage/pinning
```

Current truthful status:

```text
Core correctness: strong
HTTP contract: strong
Invariant tests: strong
Observability: good
Amnesia dev posture: good
Production auth: pending
Durable mode: pending
Holds/escrow: pending
Accounting/rewarder integration: pending
OpenAPI sync: pending
Overall: 90–93% complete
```

```
```
### BEGIN NOTE - APRIL 27 2026 - 19:55 CST


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

### END NOTE - APRIL 27 2026 - 19:55 CST



### BEGIN NOTE - APRIL 28 2026 - 23:10 CST

# svc-wallet NOTES.MD — WEB3 Mutation Boundary / Carryover Notes

Date: 2026-04-29
Crate: `svc-wallet`
Workspace path: `crates/svc-wallet`
Project: RustyOnions / WEB3 / ROC internal value plane
Session status: **Gold wallet foundation + live-proven mutation boundary for paid storage and reward issuance**
Estimated current completion: **94–97% for WEB3 beta wallet role**, **82–88% for production-grade wallet hardening**

---

## 0. Executive Summary

`svc-wallet` is now the **sole normal mutation front-door** for the internal ROC value plane. It is the crate that receives authorized economic operations, applies nonce/idempotency/policy discipline, commits through the ledger-backed adapter, and returns deterministic receipts. The WEB3 north star explicitly defines the wallet service as the API boundary for balances, transfers, issue/mint, burn, receipts, nonces, and idempotency, while `ron-ledger` remains append-only truth and `ron-accounting` remains transient metering input. 

Before this session, `svc-wallet` had already reached a strong gold plateau: strict DTOs, deterministic receipt hashing, idempotency, nonce gates, local `ron-ledger` adapter, issue/transfer/burn/balance/receipt lookup, health/ready/metrics, invariant tests, HTTP black-box tests, concurrency tests, and bench smokes. Earlier carryover notes recorded it as a strong internal ROC wallet service foundation with core correctness, HTTP contract, invariant tests, observability, and amnesia/dev posture already strong. 

This session did not mainly add new `svc-wallet` source files. Instead, it **proved `svc-wallet` under live multi-crate pressure** in two essential WEB3 flows:

```text id="0n8z59"
1. Paid storage spend path:
   issue → hold → storage verifies receipt → capture → release → balances conserve

2. Reward issuance earn path:
   accounting vector → rewarder compute → rewarder HTTP emit → wallet issue → replay no double issue
```

Those are the two directions of the ROC economy:

```text id="wcth2j"
Spend/use side:
  wallet issue/hold/capture/release supports paid storage

Earn/reward side:
  rewarder emits wallet issue requests and wallet prevents double issuance on replay
```

The important current truth: `svc-wallet` is no longer just a tested standalone service. It is now the live economic boundary used by `svc-storage`, `svc-rewarder`, and `ron-accounting`-driven value flows.

---

## 1. Strategic Role in WEB3 / ROC

`svc-wallet` is the mutation boundary.

Its correct role:

```text id="8s9fuo"
accept wallet API requests
enforce auth/capability/policy gates
enforce idempotency
enforce per-account nonce rules where required
validate strict DTOs
commit economic mutations through ledger-backed adapter
return deterministic receipts
serve balances
serve receipt lookup
expose metrics, health, and readiness
protect against replay/double issue/double spend
```

Its incorrect role — do not regress into this:

```text id="q920xj"
not raw ledger storage
not accounting/metering
not reward computation
not storage enforcement
not an external-chain bridge
not Solana/ROX logic
not staking/governance/liquidity
not dashboard state
not b3 asset ownership registry
```

Correct boundaries:

```text id="xazxxw"
ron-ledger:
  durable append-only truth, replay, balance state

svc-wallet:
  mutation front-door, API, idempotency, nonce, receipts, balances

ron-accounting:
  usage/metering/snapshots, not truth

svc-rewarder:
  deterministic reward planning, commits by calling wallet

svc-storage:
  paid-use enforcement, commits by calling wallet
```

Current proven role in live system:

```text id="q643bm"
svc-storage calls wallet for hold/capture/release.
svc-rewarder calls wallet for issue.
wallet receipts are returned and used as proof.
wallet balances are checked after mutation.
wallet idempotency prevents duplicate reward issuance.
```

---

## 2. What Was Already Working Before This Session

Before this session, `svc-wallet` had already been built into a serious internal ROC wallet foundation.

Existing capabilities from prior notes:

```text id="lqn1ij"
strict request/response DTOs
AmountMinor string-encoded value boundary
deterministic receipt DTO
WalletOp enum for issue/transfer/burn/hold/capture/release
deterministic receipt_hash
deterministic tx/request fingerprints
idempotency store
strict nonce table
capability/policy seams
local ron-ledger adapter
issue route
transfer route
burn route
hold route
capture route
release route
balance route
receipt lookup route
/healthz
/readyz
/metrics
in-process dev state
receipt book
metrics counters/gauges
readiness/shedding behavior
amnesia-mode tests
loom/concurrency tests
paid-storage smoke test
bench smokes
```

The current codebundle’s wallet response DTOs show `WalletOp` includes all six required operations: `Issue`, `Transfer`, `Burn`, `Hold`, `Capture`, and `Release`, and wallet receipts include operation, from/to, asset, amount, nonce, idempotency key, timestamp, ledger sequence bounds, ledger root, and receipt hash. 

Prior carryover notes recorded the accepted role:

```text id="6e05eu"
all ROC balance mutations must go through wallet
wallet commits through ron-ledger
wallet protects mutations with auth, idempotency, nonce rules, policy checks, and receipts
```

They also listed the major test surface: double-spend, non-negativity, conservation, caps/policy, DTO hygiene, ledger primacy, observability, amnesia, overflow ceilings, receipt vectors, HTTP black-box tests, escrow HTTP tests, loom invariants, and paid-storage smoke. 

---

## 3. What We Proved for svc-wallet in This Session

### 3.1 Wallet was live in the paid-storage three-process smoke

The paid-storage live smoke started:

```text id="ikw37w"
ron-accounting ingest on 127.0.0.1:19600
svc-wallet on 127.0.0.1:18088
svc-storage on 127.0.0.1:15303
```

The terminal output confirmed:

```text id="gvhclu"
svc-wallet is healthy
{
  "service": "svc-wallet",
  "ok": true
}
```

Then the live flow used wallet for:

```text id="slza9p"
issue ROC to acct_user
create hold from acct_user to escrow_paid_write
capture actual storage cost from escrow_paid_write to svc_storage
release remainder from escrow_paid_write back to acct_user
serve final balances
serve receipts used by storage
```

The live output showed `svc-wallet` issued `100` ROC, created a `70` ROC hold, captured `48`, released `22`, and the final balances were `acct_user = 52`, `escrow_paid_write = 0`, and `svc_storage = 48`. 

This proves wallet is working as the mutation boundary for paid use.

### 3.2 Wallet protected paid storage with hold → capture → release

The paid storage path now exercises the full escrow lifecycle:

```text id="75upv0"
hold:
  acct_user → escrow_paid_write

capture:
  escrow_paid_write → svc_storage

release:
  escrow_paid_write → acct_user
```

The live output showed the wallet hold receipt included:

```text id="o3h9yj"
op = hold
from = acct_user
to = escrow_paid_write
asset = roc
amount_minor = 70
nonce = 1
idem = storage_put:<context hash>
receipt_hash = b3:<64 lowercase hex>
```

Then the paid object response included both:

```text id="yvaun5"
capture_receipt:
  op = capture
  from = escrow_paid_write
  to = svc_storage
  amount_minor = 48

release_receipt:
  op = release
  from = escrow_paid_write
  to = acct_user
  amount_minor = 22
```

This is the canonical paid-use settlement model required by WEB3.

### 3.3 Wallet balances conserved through paid storage

The paid-storage live smoke verified final balances:

```text id="w07h47"
payer balance:  52
escrow balance: 0
payee balance:  48
```

Given:

```text id="1kt68h"
initial issue = 100
hold = 70
capture = 48
release = 22
```

Conservation check:

```text id="za88c2"
acct_user = 100 - 70 + 22 = 52
escrow_paid_write = 70 - 48 - 22 = 0
svc_storage = 48
total = 100
```

That proves the wallet/ledger path preserved value exactly for transfer-like operations.

### 3.4 Wallet receipts are now used as live admission proof by storage

The paid storage route does not merely trust a dev header in the live smoke. It verifies wallet receipts over HTTP. Prior notes highlight `"verifier": "wallet-http"` as the important proof that storage is calling `svc-wallet` for the receipt rather than just trusting synthetic metadata. 

This means `svc-wallet` receipts are now an interoperability contract:

```text id="9f130x"
wallet receipt shape
→ storage verifies receipt
→ storage validates context binding
→ storage admits paid write
```

The wallet receipt is becoming the project’s client-visible economic proof object.

### 3.5 Wallet participated in the reward issuance flow

This session also proved the earning/reward direction:

```text id="u35xsk"
ron-accounting reward vector
→ svc-rewarder compute
→ svc-rewarder HTTP emit
→ svc-wallet /v1/issue
→ ledger-backed balances
```

The final rewarder smoke proved:

```text id="g3oa8x"
acct_a = 356 ROC minor units
acct_b = 643 ROC minor units
payout_total = 999 ROC minor units
replay = no double issue
```

The corrected final run showed all rewarder tests green, then the live smoke green with `WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green`. 

From the wallet perspective, this proves:

```text id="s7u6cn"
wallet can accept rewarder-issued issue requests
wallet creates receipts for reward payouts
wallet ledger sequence/root fields are present
wallet idempotency protects reward epoch replay
```

The emitted rewarder response also showed wallet receipts for both accounts, including `op = issue`, `asset = roc`, `amount_minor = 356/643`, ledger roots, sequence ranges, receipt hashes, and txids. 

### 3.6 Wallet idempotency prevented reward double issue

The earning-side smoke did this:

```text id="eh07my"
emit reward payout batch once
emit same reward payout batch again
check wallet balances
```

Final proof:

```text id="4bf1ds"
replay = no double issue
```

This is one of the most important WEB3 wallet invariants. Rewarder duplicate detection is helpful, but wallet idempotency is the hard final defense. The current live proof shows that replaying the reward emit does not mint/issue additional ROC.

### 3.7 Wallet remains the shared mutation front-door for both spend and earn sides

This session proved wallet is used by:

```text id="6lidof"
svc-storage for hold/capture/release
svc-rewarder for issue
manual/dev smoke for issue and balance checks
```

This means wallet has become the central value-plane bridge between services and ledger:

```text id="n9nkn9"
paid storage spends ROC through wallet
rewarder issues ROC through wallet
balance visibility comes from wallet
receipts come from wallet
ledger roots/sequences come through wallet receipts
```

That is exactly what WEB3 requires.

---

## 4. Current Proven Wallet API Surface

Current API surface from notes and tests:

```text id="ox2cal"
GET  /healthz
GET  /readyz
GET  /metrics

GET  /v1/balance
GET  /v1/tx/:txid

POST /v1/issue
POST /v1/transfer
POST /v1/burn
POST /v1/hold
POST /v1/capture
POST /v1/release
```

### 4.1 Issue

Used in both live smokes:

```text id="z0sjoe"
paid storage smoke:
  issue 100 ROC to acct_user

rewarder smoke:
  issue 356 ROC to acct_a
  issue 643 ROC to acct_b
```

Issue is the explicit supply-creating exception. It must remain tightly authorized and auditable.

### 4.2 Transfer

Already covered by existing tests and basic wallet core. Used for ordinary account-to-account movement.

### 4.3 Burn

Already covered by existing tests. Burn is the explicit supply-destroying exception. It must remain tightly authorized and auditable.

### 4.4 Hold

Used in paid storage. Reserves value by moving payer funds into escrow.

### 4.5 Capture

Used in paid storage. Pays the service/provider out of escrow after successful operation.

### 4.6 Release

Used in paid storage. Refunds unused escrow remainder back to payer.

### 4.7 Balance

Used by smokes to verify final economic state.

### 4.8 Receipt lookup

Used by storage receipt verification and tests. This is now crucial for service-to-service paid-use enforcement.

---

## 5. Current Receipt Contract

Wallet receipts currently serve as the proof layer for:

```text id="gupk8k"
issue
transfer
burn
hold
capture
release
```

Important fields:

```text id="mtcp3c"
txid
op
from
to
asset
amount_minor
nonce
idem
timestamp
ledger_seq_start
ledger_seq_end
ledger_root
receipt_hash
```

The DTO code shows `amount_minor` is string-encoded, `receipt_hash` is computed over canonical fields excluding the hash itself, and operations have stable snake_case labels. 

Receipt invariants:

```text id="b821bl"
receipt_hash must be b3:<64 lowercase hex>
receipt op must match operation
asset must be roc for ROC operations
amount_minor must be integer string
ledger sequence/root should bind the receipt to ledger progress
idempotency key should be echoed as idem
txid should be deterministic/stable enough for lookup
```

Current status:

```text id="pc5djh"
storage can verify hold receipt
storage can validate capture/release receipt
rewarder can collect issue receipts
users/services can retrieve receipts by txid
```

---

## 6. Quality Gates Currently Associated With svc-wallet

Prior gold gate:

```bash id="kkkfzt"
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
cargo run -p svc-wallet
```

Session-level multi-crate gates involving wallet:

```bash id="6t7t4u"
bash scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

The paid storage smoke proved:

```text id="pt81wb"
svc-wallet starts healthy
issue works
hold works
receipt lookup works
storage can verify receipt
capture works
release works
balance reads work
final balances conserve
```

The rewarder smoke proved:

```text id="u8v5p5"
svc-wallet starts healthy
rewarder can issue through wallet
wallet returns issue receipts
wallet balances update
re-emitting the same reward settlement does not double issue
```

Recommended standing wallet-specific gate:

```bash id="z8pfje"
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
```

Recommended integration gate after wallet changes:

```bash id="o3bkwi"
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

---

## 7. Current Completion Estimate

After this session:

```text id="jmiqz5"
wallet core DTO/API foundation:             95–98%
issue/transfer/burn path:                   94–97%
hold/capture/release escrow path:           95–98%
receipt generation and lookup:              94–97%
idempotency/replay protection:              94–97%
nonce discipline:                           90–95%
metrics/health/ready:                       85–92%
paid storage wallet integration:            97–99%
rewarder wallet integration:                96–98%
production auth/capability integration:     60–70%
durable idempotency/restart behavior:       65–78%
dashboard/admin receipt visibility:         55–70%
overall WEB3 beta wallet readiness:         94–97%
production-grade wallet readiness:          82–88%
```

Interpretation:

```text id="1ejig0"
For WEB3 beta:
  svc-wallet is essentially ready as the internal ROC mutation boundary.

For production:
  the big unfinished pieces are durable idempotency/restart semantics,
  real ron-auth/svc-passport capability enforcement, config hardening,
  admin/dashboard receipt visibility, and persistence/recovery policy.
```

---

## 8. Important Invariants to Preserve

Do not regress:

```text id="quoy0f"
all economic mutation goes through svc-wallet
svc-wallet commits through ron-ledger
ron-ledger remains durable truth
wallet does not become accounting
wallet does not compute rewards
wallet does not enforce storage semantics beyond wallet policy/caps
no floating-point money math
all ROC amounts are integer minor units
amounts are string encoded at JSON boundaries where required
no negative balances by default
transfer-like operations conserve value
issue is explicit supply creation
burn is explicit supply destruction
hold/capture/release conserve value through escrow
capture cannot exceed held amount
release cannot exceed held amount/remainder
failed capture must not credit payee
failed ledger commit must roll back nonce reservation
same idempotency key + same fingerprint returns same result
same idempotency key + different body conflicts
nonce replay rejects
receipt_hash is deterministic
receipt lookup returns original receipt
metrics must reflect accepted/replay/rejected ops truthfully
healthz/readyz must not lie
amnesia/dev mode must be explicit
no ROX/Solana/external settlement logic
```

Critical paid-use invariant:

```text id="77ydxv"
hold → capture → release is the canonical paid operation state machine.
```

Critical reward invariant:

```text id="lqxzho"
rewarder may plan/emit, but wallet idempotency must prevent double issue.
```

---

## 9. What Remains for svc-wallet

### Priority A — Real capability/auth integration

Current dev smokes use bearer-style auth such as:

```text id="hn19sd"
Authorization: Bearer dev
```

Production needs:

```text id="23oa6l"
ron-auth capability verifier
svc-passport identity integration
scoped wallet capabilities
short TTL tokens
issuer/audience validation
account caveats
asset caveats
amount ceilings
operation caveats
epoch/reward caveats for issue
paid-storage caveats for hold/capture/release
fail-closed defaults
```

Recommended scopes:

```text id="wk8voo"
wallet.balance.read
wallet.receipt.read
wallet.issue
wallet.transfer
wallet.burn
wallet.hold
wallet.capture
wallet.release
wallet.admin
```

Issue and burn must be the most restricted scopes.

### Priority B — Durable idempotency decision

Current idempotency is strong in dev/live smokes. Production must define persistence.

Open decisions:

```text id="jq8930"
Is idempotency store in-memory for amnesia mode only?
What durable backend is used for beta?
What is the retention window?
How are old idempotency keys compacted?
How are conflicts retained across restart?
Can receipt lookup recover idempotent results after restart?
What metrics report idempotency-store health?
```

Needed tests:

```text id="ctu7ix"
issue request succeeds
service restarts
same idempotency key/body returns same receipt
same idempotency key/different body still conflicts
receipt lookup still works
metrics reflect replay after restart
```

### Priority C — Durable receipt book / receipt indexing

Current receipt lookup works in dev/live path. Production needs durable receipt retrieval.

Needed:

```text id="rco21d"
receipt storage policy
receipt retention policy
receipt export format
receipt lookup after restart
receipt lookup by txid
possibly lookup by idempotency key
possibly lookup by account/time/op
dashboard-friendly receipt index
```

Receipt storage must not become a second ledger. Ledger remains truth.

### Priority D — Config loader hardening

Wallet needs a production-grade config model for:

```text id="5a48l5"
bind address
metrics address
body size limits
timeout limits
max inflight
rate limits
amnesia mode
ledger backend path/mode
idempotency backend
receipt backend
auth/capability mode
allowed assets
issue/burn policies
default ceilings
escrow account rules
```

Desired precedence:

```text id="kq4hxp"
defaults → config file → env → CLI
```

### Priority E — OpenAPI / schema sync

Wallet API is now important enough to need a stable schema.

Needed artifacts:

```text id="79f9vx"
OpenAPI spec for /v1 routes
JSON schemas for request DTOs
JSON schemas for receipt DTOs
examples for issue/transfer/burn/hold/capture/release
error response schema
idempotency conflict schema
nonce conflict schema
metrics contract
```

This will help dashboard, browser extension, gateway, rewarder, storage, and future SDKs.

### Priority F — Production ledger persistence/replay integration

Wallet currently commits through ledger-backed adapter and exposes ledger roots/sequence fields. Production needs restart proof:

```text id="4zygx8"
start wallet with durable ledger
issue/hold/capture/release
stop process
restart process
balance reads match pre-restart state
receipt lookup works or can be reconstructed
ledger replay validates roots/sequences
idempotency still prevents replay
```

This overlaps with `ron-ledger` hardening.

### Priority G — Reconciliation / repair tools

Needed admin tools:

```text id="1nzwhb"
list recent receipts
verify receipt hash
verify ledger sequence continuity
verify account balance from ledger replay
find escrow positions
list open holds
reconcile capture/release pairs
identify orphaned escrow balances
manual release/reconcile path under admin capability
```

This matters for paid storage and future paid content.

### Priority H — Escrow lifecycle visibility

Paid storage now uses escrow. Wallet should expose safe visibility:

```text id="vk99v7"
GET /v1/escrow/:account
GET /v1/holds?account=...
GET /v1/holds?escrow=...
GET /v1/holds?status=open|captured|released
```

Only if it can be done without becoming an application-specific storage service. It should expose generic escrow state, not storage semantics.

### Priority I — Stronger account canonicalization

Current tests validate allowed account chars. Production should define canonical account IDs across:

```text id="p7oe3q"
wallet accounts
passport accounts
service accounts
escrow accounts
treasury accounts
reward pool accounts
provider payout accounts
anonymous/alternate passport accounts
```

Needed:

```text id="01ms5o"
max length
allowed chars
tenant/account namespace
reserved prefixes
service account format
escrow account format
treasury account format
case sensitivity
migration/alias policy
```

### Priority J — Rate limiting and abuse resistance

Wallet will be high-value.

Needed:

```text id="gml7zj"
per-token request limits
per-account mutation limits
per-IP dev limits
max body
max decompression ratio
max amount
daily caps through policy/capability
reject storms metrics
shed under overload
```

### Priority K — Dashboard/admin visibility

Dashboard should eventually show:

```text id="vgrfke"
total supply
issued total
burned total
transfer volume
hold volume
capture volume
release volume
open escrow total
idempotency replays
idempotency conflicts
nonce conflicts
top service accounts
recent receipts
ledger height
ledger root
readiness status
```

Golden metrics likely needed:

```text id="l2qmj3"
wallet_ops_total{op}
wallet_rejected_total{reason}
wallet_idempotency_replays_total
wallet_nonce_rejects_total
wallet_receipts_total{op}
wallet_balance_read_total
wallet_ledger_commit_seconds
wallet_inflight
wallet_ready
wallet_escrow_open_amount_minor
```

### Priority L — Rewarder capability and supply policy

Since `svc-rewarder` now emits to wallet, wallet needs explicit supply policy:

```text id="kpjr5j"
which service can issue ROC?
which policy hash authorizes reward issue?
which epoch/run key is allowed?
what max amount per epoch?
what max issue per account?
what max issue per service?
what happens on duplicate epoch?
how are issue receipts surfaced?
```

Wallet should not blindly accept broad issue capability in production.

### Priority M — Paid-storage-specific policy remains outside wallet

Important: wallet should support generic holds/captures/releases. It should not become aware of `paid_storage_put`, object CIDs, pinning semantics, or b3 asset ownership except as opaque memo/idempotency/context fields.

Storage/gateway/omnigate policy belongs upstream.

---

## 10. Known Commands

### Full wallet gate

```bash id="hqq2c6"
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
```

### Run wallet manually

```bash id="s4j8fc"
SVC_WALLET_ADDR=127.0.0.1:18088 \
cargo run -p svc-wallet
```

### Health/readiness/metrics

```bash id="xr5wx7"
curl -fsS http://127.0.0.1:18088/healthz | jq .
curl -fsS http://127.0.0.1:18088/readyz | jq .
curl -fsS http://127.0.0.1:18088/metrics | head
```

### Issue

```bash id="sgx3ls"
curl -fsS -X POST http://127.0.0.1:18088/v1/issue \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: idem_issue_1' \
  -H 'Content-Type: application/json' \
  -d '{"to":"acct_user","asset":"roc","amount_minor":"100","memo":"manual issue"}' \
  | jq .
```

### Balance

```bash id="g62eq9"
curl -fsS \
  'http://127.0.0.1:18088/v1/balance?account=acct_user&asset=roc' \
  -H 'Authorization: Bearer dev' \
  | jq .
```

### Hold

```bash id="g69gav"
curl -fsS -X POST http://127.0.0.1:18088/v1/hold \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: storage_put:example' \
  -H 'Content-Type: application/json' \
  -d '{"from":"acct_user","to":"escrow_paid_write","asset":"roc","amount_minor":"70","nonce":1,"memo":"manual hold"}' \
  | jq .
```

### Capture

```bash id="sjahg4"
curl -fsS -X POST http://127.0.0.1:18088/v1/capture \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: storage_cap:example' \
  -H 'Content-Type: application/json' \
  -d '{"from":"escrow_paid_write","to":"svc_storage","asset":"roc","amount_minor":"48","nonce":1,"memo":"manual capture"}' \
  | jq .
```

### Release

```bash id="qf66ps"
curl -fsS -X POST http://127.0.0.1:18088/v1/release \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: storage_rel:example' \
  -H 'Content-Type: application/json' \
  -d '{"from":"escrow_paid_write","to":"acct_user","asset":"roc","amount_minor":"22","nonce":2,"memo":"manual release"}' \
  | jq .
```

### Receipt lookup

```bash id="bng7p0"
curl -fsS \
  http://127.0.0.1:18088/v1/tx/<txid> \
  -H 'Authorization: Bearer dev' \
  | jq .
```

### Paid storage live smoke

```bash id="e5cuzq"
bash scripts/web3_paid_storage_live_smoke.sh
```

Expected final line:

```text id="fwow8d"
WEB3 paid-storage live smoke with settlement + real accounting export green
```

### Rewarder wallet emit live smoke

```bash id="3c3ydj"
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Expected final line:

```text id="2mwr6p"
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```

---

## 11. Suggested NOTES.MD Commit Summary

```text id="tsjgji"
svc-wallet: document live WEB3 mutation-boundary proof

- record existing gold wallet foundation: strict DTOs, idempotency, nonce gates,
  deterministic receipts, ledger-backed adapter, /v1 issue/transfer/burn/hold/capture/release,
  balance reads, receipt lookup, metrics, health, and readiness
- document paid-storage live use: issue, hold, storage receipt verification,
  capture, release, balance conservation, and receipt proof
- document rewarder live use: rewarder emits issue requests to wallet,
  wallet returns receipts, balances update, and replay does not double issue
- preserve boundary: wallet is mutation front-door; ledger is truth; accounting/rewarder/storage do not mutate ledger directly
- record remaining work: real capabilities, durable idempotency, durable receipts,
  OpenAPI/schema sync, restart replay, dashboard/admin visibility, and production auth
```

Longer body:

```text id="jr1fe0"
svc-wallet is now the live-proven mutation boundary for the RustyOnions internal ROC economy.
It supports both sides of the value plane: the paid-use spend path through hold/capture/release
and the reward-issuance earn path through issue. The paid-storage live smoke proves wallet
issue, hold, capture, release, receipt verification, balance conservation, and settlement
receipts. The rewarder live smoke proves rewarder-issued wallet issues update balances and
replay does not double issue.

The crate remains correctly bounded: it exposes wallet mutations and receipts, commits through
ron-ledger, and does not become accounting, rewarder, storage policy, or external-chain logic.
```

---

## 12. Final Status

`svc-wallet` should currently be considered:

```text id="ketzlo"
Status: Gold foundation + live-proven WEB3 mutation front-door
WEB3 beta readiness: 94–97%
Production hardening readiness: 82–88%
Main remaining work: real auth/capabilities, durable idempotency/receipts,
restart replay, config hardening, OpenAPI/schema sync, reconciliation/admin tools,
dashboard visibility, and production supply policy.
```

Most important current achievement:

```text id="zveqfm"
svc-wallet now powers both the spend side and earn side of ROC:

Spend:
  issue → hold → paid storage → capture → release → balances conserve

Earn:
  accounting → rewarder → wallet issue → balances update → replay no double issue
```

That makes `svc-wallet` one of the most important completed pillars of the RustyOnions WEB3 beta.


### END NOTE - APRIL 28 2026 - 23:10 CST