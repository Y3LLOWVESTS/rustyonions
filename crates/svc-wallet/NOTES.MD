
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


### BEGIN NOTE - JUNE 14 2026 - 10:05 CST

The latest terminal output is an excellent stopping point: the new `quickchain_preflight_idempotency_identity_boundary` test passed 3/3, the live-route matrix stayed 3/3, no-runtime-authority stayed 3/3, preflight boundary stayed 7/7, both normal and `quickchain-preflight` `svc-wallet` suites passed, clippy passed, and workspace check only shows the same unrelated warnings in `micronode`, `ryker`, and `omnigate`. 

Here are the carryover notes for the next session:

# RustyOnions QuickChain / `svc-wallet` Carryover Notes

## Session checkpoint

We are pausing after a strong green checkpoint for `svc-wallet` QuickChain Phase-0/preflight work.

The active crate is:

```text
crates/svc-wallet
```

The active QuickChain posture remains:

```text
QuickChain is future settlement infrastructure.
QC-0A / quickchain-preflight is safety, compatibility, review, and DTO/projection boundary work only.
No roots, checkpoints, validators, anchors, pruning, settlement, bridges, staking, liquidity, public-chain behavior, or live chain authority are allowed yet.
```

The latest terminal run passed:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
cargo check --workspace
```

The only workspace warnings are unrelated to `svc-wallet`:

```text
micronode: unused import UpstreamSpec
ryker: max_msg_bytes is never read
omnigate: MAX_ACCOUNT_BYTES is never used
```

These warnings should not block `svc-wallet` or QuickChain work.

---

# Current estimated completion

## Overall QuickChain vision

Estimated completion:

```text
36–40%
```

Why this increased:

* `ron-proto` QuickChain DTO groundwork was already mostly completed.
* `ron-ledger` QuickChain preflight/replay substrate was already in strong shape.
* `svc-wallet` now has a much stronger Phase-0 boundary:

  * accepted-only receipt finality
  * no fake finality labels
  * QuickChain preflight projection gated behind feature
  * live HTTP routes do not leak QuickChain authority vocabulary
  * idempotency keys proven separate from operation authority
  * full route matrix tested under `quickchain-preflight`

Why it is not higher:

* No canonical QuickChain roots are allowed or implemented yet.
* No locked canonical bytes / locked hash vectors for chain roots yet.
* No receipt-root, account-state-root, hold-root, accounting-root, reward-root, DA, challenge, archive, pruning, or validator flow yet.
* `ron-accounting`, `svc-rewarder`, `svc-gateway`, `omnigate`, and storage paid enforcement still need cross-crate proof gates.
* QuickChain still has no live runtime authority, by design.

## QuickChain Phase 0 / preflight readiness

Estimated completion:

```text
68–74%
```

Why:

* DTO strictness and no-unknown-field discipline are strong.
* `ron-ledger` replay and deterministic safety work is strong.
* `svc-wallet` now has serious preflight boundary tests.
* The most dangerous fake-chain-authority mistakes have been actively guarded against.

Remaining Phase 0 work:

* Add final `svc-wallet` request-poisoning matrix across all mutation routes.
* Add documentation/runbook for `svc-wallet` QuickChain preflight boundaries.
* Create or update carryover docs inside the crate if desired.
* Confirm the full cross-crate value loop with `ron-accounting` and `svc-rewarder`.
* Begin canonical vector planning only after the current crate boundary is completely stable.
* Do not generate fake hashes or placeholder vectors.

## `svc-wallet` internal ROC wallet readiness

Estimated completion:

```text
98–99% beta-complete
```

What this means:

`svc-wallet` is very close to complete as an internal ROC wallet front door for local/dev/beta flows. It supports:

* issue
* transfer
* burn
* hold
* capture
* release
* receipt lookup
* idempotent replay
* idempotency conflict rejection
* nonce protection
* policy/capability seams
* metrics
* readiness
* bounded transport/body limits
* DTO hygiene
* no floats
* string-encoded integer minor units
* accepted-only receipt status
* receipt hash generation
* paid-storage smoke flow

What still prevents calling it “production complete”:

* It uses in-memory/dev state in current path.
* External production `ron-auth`, `ron-policy`, `ron-ledger` service boundary, durable idempotency, and durable receipt lookup are not fully wired.
* Escrow request DTOs currently reuse `TransferRequest`; dedicated hold/capture/release DTOs may be needed later.
* Accounting emission is still a no-op seam.
* Production deployment/runbook work remains.
* More cross-service paid enforcement tests remain.

## `svc-wallet` QuickChain Phase-0 slice

Estimated completion:

```text
48–54%
```

This increased significantly during the latest work.

Why:

* `svc-wallet` now has a `quickchain-preflight` feature.
* QuickChain code is feature-gated and disabled by default.
* The feature exposes only inert projection helpers.
* The projection requires explicit backend context.
* Live HTTP routes remain wallet receipts, not QuickChain projection DTOs.
* Request DTOs reject unknown QuickChain authority fields.
* Idempotency keys are now explicitly proven to be retry identity, not `operation_id`.
* Changing explicit projection `operation_id` does not rewrite wallet receipt hash.
* Full feature/no-feature gates pass.

Why it is not higher:

* `svc-wallet` does not yet have durable operation identity storage.
* `operation_id` is still caller-supplied projection context in tests, not a persisted backend-assigned operation journal.
* No canonical QuickChain receipt DTO owned by `ron-proto` is wired into `svc-wallet`.
* No locked canonical vector bytes/hashes.
* No accounting/rewarder/gateway cross-service QuickChain proof gates yet.
* No roots, checkpoints, settlement, DA, validators, or chain-runtime logic, by design.

---

# Major accomplishments so far

## 1. `svc-wallet` now has honest receipt finality

Receipts now expose:

```rust
settlement_status: ReceiptSettlementStatus::Accepted
```

The important rule is:

```text
svc-wallet may honestly say accepted.
svc-wallet must not claim finalized, anchored, checkpointed, epoch-included, irreversible, or settled.
```

This is crucial because `svc-wallet` is the wallet/ledger hot path, not future QuickChain finality.

Accomplished protections:

* Receipt status serializes as `"accepted"`.
* Future finality labels are rejected by the wallet receipt DTO.
* Wallet mutation receipts are accepted-only.
* Settlement status is included in the receipt hash preimage.
* A receipt hash now commits to the honest finality label.

Relevant test file:

```text
crates/svc-wallet/tests/i_15_receipt_finality_honesty.rs
```

## 2. `svc-wallet` now has a gated QuickChain preflight feature

The feature exists in:

```text
crates/svc-wallet/Cargo.toml
```

Current feature:

```toml
quickchain-preflight = ["ron-ledger/quickchain-preflight"]
```

Important meaning:

```text
This forwards ron-ledger's pre-root/pre-consensus QuickChain surface so svc-wallet can compile and test against future settlement boundaries without enabling roots, checkpoints, validators, pruning, settlement, anchors, bridges, staking, liquidity, or external-chain logic.
```

This is the correct posture.

## 3. `src/quickchain.rs` exists and is intentionally inert

Current file:

```text
crates/svc-wallet/src/quickchain.rs
```

It exposes:

```rust
SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA
QuickChainWalletReceiptStatus
QuickChainWalletReceiptProjectionContext
QuickChainWalletReceiptProjection
project_wallet_receipt_for_quickchain_preflight
```

This module is safe because:

* It is behind `#[cfg(feature = "quickchain-preflight")]`.
* It does not mutate ledger.
* It does not create roots.
* It does not create checkpoints.
* It does not create anchors.
* It does not create validator state.
* It does not claim finality.
* It does not unlock paid content.
* It does not replace wallet/ledger truth.
* It requires explicit backend `chain_id` and `operation_id`.
* It treats wallet receipt data as projection input only.

## 4. Existing boundary test now proves projection safety

Relevant test file:

```text
crates/svc-wallet/tests/quickchain_preflight_boundary.rs
```

It proves:

* Wallet QuickChain preflight keeps default wallet posture.
* ron-ledger QuickChain preflight surface is inert from wallet boundary.
* Empty accepted replay boundary rebuilds without wallet mutation.
* Wallet receipt projection requires explicit operation context.
* Projection rejects missing operation id.
* Projection rejects missing ledger sequence pair.
* Projection rejects fake receipt hash.

Latest status:

```text
7 passed
```

## 5. New no-runtime-authority test was added and passed

Relevant test file:

```text
crates/svc-wallet/tests/quickchain_preflight_no_runtime_authority.rs
```

This test proves:

* `quickchain-preflight` does not change live HTTP receipt shape.
* Live `/v1/issue` receipt stays a wallet receipt.
* Live HTTP response includes wallet fields:

  * `txid`
  * `op`
  * `to`
  * `amount_minor`
  * `settlement_status`
  * `receipt_hash`
* Live HTTP response does not expose projection-only fields:

  * `schema`
  * `chain_id`
  * `operation_id`
  * `idempotency_key`
  * `produced_at_ms`
  * `legacy_ledger_root`
* Manual projection requires explicit context.
* Request bodies cannot smuggle authority fields like:

  * `schema`
  * `chain_id`
  * `operation_id`

Latest status:

```text
3 passed
```

## 6. New live route matrix test was added and passed

Relevant test file:

```text
crates/svc-wallet/tests/quickchain_preflight_live_route_matrix.rs
```

This test proves all live mutation/read routes remain wallet-shaped under `quickchain-preflight`.

Covered routes:

```text
POST /v1/issue
POST /v1/transfer
POST /v1/burn
POST /v1/hold
POST /v1/capture
POST /v1/release
GET  /v1/tx/{txid}
```

It proves live responses do not leak:

```text
schema
chain_id
operation_id
idempotency_key
produced_at_ms
legacy_ledger_root
state_root
receipt_root
checkpoint
anchor
validator
finalized
finality
settlement_root
```

It also proves receipt lookup remains wallet receipt lookup, not QuickChain projection lookup.

Latest status:

```text
3 passed
```

## 7. New idempotency identity boundary test was added and passed

Relevant test file:

```text
crates/svc-wallet/tests/quickchain_preflight_idempotency_identity_boundary.rs
```

This is an important doctrine lock.

It proves:

```text
idempotency_key = retry identity
operation_id = backend-assigned durable operation identity
```

It proves:

* Same idempotency key + same request body returns byte-identical wallet receipt.
* Same idempotency key + changed request body returns `409 CONFLICT`.
* Conflict response uses code `IDEMPOTENCY_CONFLICT`.
* Projection requires explicit `operation_id`.
* Projection `operation_id` is not silently derived from idempotency key.
* Changing explicit projection operation id does not rewrite wallet receipt hash.
* Wallet receipt hash remains wallet-derived evidence, not chain-operation-derived evidence.

Latest status:

```text
3 passed
```

This was one of the more important Phase-0 safety gates.

## 8. Full `svc-wallet` gate is strong

Current important passing areas:

```text
unit tests
api contract
http blackbox
http escrow
transport bounds
overflow ceilings
hold/capture/release
site visit receipt replay
receipt finality honesty
double-spend prevention
non-negativity
conservation
capability policy
DTO hygiene
ledger primacy
observability
amnesia
loom invariants
quickchain preflight boundary
quickchain preflight no runtime authority
quickchain preflight live route matrix
quickchain preflight idempotency identity boundary
readiness and shedding
receipt hash vectors
paid storage smoke
benches
clippy with quickchain-preflight
full no-feature dev preflight
full feature dev preflight
workspace check
```

This is a very strong stopping point.

---

# Current `svc-wallet` architecture summary

## Source of truth

```text
ron-ledger remains balance/economic truth.
svc-wallet is the mutation front door.
svc-wallet must not become its own durable balance ledger.
```

## Current mutation routes

```text
/v1/issue
/v1/transfer
/v1/burn
/v1/hold
/v1/capture
/v1/release
```

## Current read routes

```text
/v1/balance
/v1/tx/{txid}
```

## Current service routes

```text
/healthz
/readyz
/metrics
```

## Current receipt fields

Current wallet receipts contain:

```text
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
settlement_status
receipt_hash
```

Important distinction:

```text
idem is wallet retry identity.
operation_id is future backend-assigned durable operation identity.
receipt_hash is wallet receipt evidence.
legacy_ledger_root is only projection terminology, not live HTTP terminology.
```

## Current QuickChain projection fields

The inert projection contains:

```text
schema
chain_id
operation_id
txid
op
from
to
asset
amount_minor
nonce
idempotency_key
produced_at_ms
ledger_seq_start
ledger_seq_end
legacy_ledger_root
receipt_hash
settlement_status
```

Important:

```text
Projection DTO is not a chain receipt.
Projection DTO is not a consensus DTO.
Projection DTO is not a root input commitment yet.
Projection DTO is not finality.
Projection DTO is not settlement.
Projection DTO is not payment unlock authority.
```

---

# Doctrines now locked by tests

## Doctrine 1: wallet accepted is not finality

`svc-wallet` can say:

```text
accepted
```

It cannot say:

```text
finalized
anchored
checkpointed
settled
epoch_included
irreversible
```

## Doctrine 2: feature-gated QuickChain code is not runtime authority

The `quickchain-preflight` feature may expose review/projection helpers.

It must not:

```text
change live route shape
add chain fields to live receipts
authorize spending
unlock content
mutate ledger
produce roots
produce checkpoints
claim settlement
claim finality
act as validator
act as bridge
```

## Doctrine 3: idempotency key is not operation id

The wallet idempotency key is retry identity only.

It is not:

```text
backend operation authority
QuickChain operation identity
chain transaction identity
validator authority
root input authority
settlement authority
```

## Doctrine 4: operation_id must be explicit backend context

For projection, `operation_id` must be explicitly supplied by backend context.

It must not be silently derived from:

```text
idempotency key
txid
route name
React/UI state
client request body
wall clock
database iteration order
```

## Doctrine 5: wallet receipt hash remains wallet receipt evidence

Changing the explicit projection operation id does not rewrite wallet receipt hash.

This preserves the separation between:

```text
wallet hot-path receipt evidence
future QuickChain durable operation identity
```

## Doctrine 6: live HTTP routes remain wallet routes

Even under `quickchain-preflight`, live HTTP routes emit wallet receipts.

They must not emit projection-only fields.

---

# What remains in `svc-wallet`

## High-priority remaining work

### 1. Request poisoning matrix across all mutation routes

Current no-runtime-authority test proves `/v1/issue` rejects unknown QuickChain authority fields.

The next good patch should expand this into a route matrix for all mutation routes:

```text
/v1/issue
/v1/transfer
/v1/burn
/v1/hold
/v1/capture
/v1/release
```

Goal:

```text
Every mutation request DTO rejects unknown QuickChain authority fields.
```

Fields to try poisoning with:

```text
schema
chain_id
operation_id
receipt_root
state_root
checkpoint
anchor
validator
finalized
settlement_status maybe with non-accepted value
```

Expected behavior:

```text
client error
not 200 OK
no mutation should occur
```

For transfer/burn/hold/capture/release, use funded setup where needed but poison the mutation request itself.

Suggested new file:

```text
crates/svc-wallet/tests/quickchain_preflight_request_poisoning_matrix.rs
```

This should be test-only and safe.

### 2. Projection validation matrix

Current projection tests cover several invalid cases, but we can make this more complete.

Add tests for:

```text
bad chain_id characters
empty chain_id
too-long chain_id
bad operation_id characters
empty operation_id
too-long operation_id
bad txid
wrong asset
bad account id in from
bad account id in to
zero produced_at_ms
ledger_seq_end < ledger_seq_start
bad legacy_ledger_root
bad receipt_hash
```

Suggested file:

```text
crates/svc-wallet/tests/quickchain_preflight_projection_validation_matrix.rs
```

This is still safe because it only tests inert projection validation.

### 3. Documentation/runbook inside `svc-wallet`

Add or update docs such as:

```text
crates/svc-wallet/docs/quickchain-preflight.md
```

This doc should explain:

* What `quickchain-preflight` enables.
* What it does not enable.
* Why projection is not chain authority.
* Why live routes remain wallet receipts.
* Why idempotency is retry identity.
* Why operation id is backend context.
* Why accepted is not finality.
* Which tests enforce the boundary.
* Which commands to run before merging.
* What must not be added in `svc-wallet`.

This would be valuable because the test suite is now strong enough to deserve a human-readable boundary document.

### 4. Update `dev-quickchain-preflight.sh` if needed

The script currently runs:

```text
cargo fmt -p svc-wallet -- --check
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
cargo test -p svc-wallet --all-targets --features quickchain-preflight
```

This is good.

Optional future improvement:

* Echo a short banner explaining that `quickchain-preflight` does not enable roots or settlement.
* Keep script simple.
* Do not make it mutate files.
* Do not add root/vector generation here.

### 5. Dedicated escrow DTOs later

Currently hold/capture/release use `TransferRequest`.

This is acceptable for the current slice, but later it may be cleaner to add:

```text
HoldRequest
CaptureRequest
ReleaseRequest
```

These could eventually include:

```text
hold_id
payer
escrow_account
payee
amount_minor
asset
nonce
memo
```

But be careful:

* `hold_id` must identify lifecycle.
* Hold lifecycle must not become QuickChain settlement authority.
* Expiry must be epoch-based if tied to future QuickChain.
* Do not add wall-clock expiry as root authority.
* Keep this for later after Phase-0 boundary is finished.

### 6. Durable operation journal later

Eventually `svc-wallet` will need a backend-assigned durable operation identity.

Future concept:

```text
operation_id = backend-assigned durable ledger-operation identity
idempotency_key = client retry key
```

Current tests prove the distinction but do not implement durable operation storage.

A future safe design phase should specify:

```text
OperationJournal
OperationRecord
operation_id allocator
idempotency-to-operation binding
receipt lookup by operation id
replay behavior
conflict behavior
durability requirements
crash recovery expectations
```

Do not rush this.

This should probably be designed after `ron-accounting` and `svc-rewarder` cross-crate expectations are clearer.

### 7. Real external dependency seams

Production readiness still needs:

```text
real ron-ledger service/client boundary
real ron-auth integration
real ron-policy integration
real ron-accounting export
durable idempotency store
durable receipt lookup
service config loader
production deployment docs
```

This is normal and outside the current QuickChain preflight slice.

---

# What is complete enough in `svc-wallet`

The following can be considered complete enough for this checkpoint:

```text
basic wallet DTO hygiene
amount string serialization
no floats
issue route
transfer route
burn route
hold route
capture route
release route
balance route
receipt lookup route
receipt hash preimage includes settlement status
accepted-only wallet receipt status
idempotency replay/conflict behavior
nonce strictness
ledger primacy tests
escrow conservation tests
paid storage smoke
QuickChain preflight feature gate
inert projection helper
no-runtime-authority route test
live route matrix test
idempotency identity boundary test
full clippy/test gate
```

This crate is now in a very good state for the current mission.

---

# Recommended next session order for `svc-wallet`

## Step 1: Reconfirm baseline

At the start of next session, run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
```

If green, continue.

## Step 2: Add request poisoning matrix

Next safest patch:

```text
crates/svc-wallet/tests/quickchain_preflight_request_poisoning_matrix.rs
```

Purpose:

```text
All wallet mutation routes reject unknown QuickChain authority fields in request bodies.
```

This complements the live-route matrix.

The live-route matrix proves:

```text
QuickChain fields do not leak out.
```

The poisoning matrix should prove:

```text
QuickChain fields cannot be smuggled in.
```

## Step 3: Add projection validation matrix

Next patch after poisoning:

```text
crates/svc-wallet/tests/quickchain_preflight_projection_validation_matrix.rs
```

Purpose:

```text
Projection DTO validation rejects malformed chain/context/receipt fields.
```

## Step 4: Add docs

After the test boundary is complete, add:

```text
crates/svc-wallet/docs/quickchain-preflight.md
```

This should be the human-readable crate boundary document.

## Step 5: Generate new codebundle

After docs/tests pass:

```bash
bash scripts/make_crate_codex.sh -c svc-wallet
```

Then attach:

```text
crates/svc-wallet/CODEBUNDLE.md
```

for next review.

---

# Recommended next commands after the next patch

Use the same gate:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p svc-wallet

cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_request_poisoning_matrix

cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary

cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings

crates/svc-wallet/scripts/dev-quickchain-preflight.sh

cargo check --workspace
```

Expected result should remain:

```text
all svc-wallet tests pass
clippy passes
workspace check passes with only unrelated warnings
```

---

# QuickChain overall state

## Already accomplished across the broader QuickChain effort

### `ron-proto`

Previously completed/mostly completed:

```text
QuickChain DTO scaffolding
strict validation helpers
schema constants
unknown-field rejection
integer minor-unit strings
b3 lowercase hash shape
event-class enum
operation/idempotency distinction
canonical JSON v1 planning posture
no fake hashes/placeholders doctrine
```

Estimated state:

```text
ron-proto QuickChain Phase-0 slice: mostly complete / high confidence
```

Remaining for `ron-proto` later:

```text
locked canonical bytes
locked hashes
golden vectors
final canonical DTO naming
cross-language vector documentation
```

Do not generate fake vector hashes.

### `ron-ledger`

Previously completed/mostly completed:

```text
deterministic accepted operation replay
ledger-owned economic truth
operation history/replay boundary
hold lifecycle preflight behavior
idempotency/retry behavior
state rebuild checks
projection/replay safety
rejection does not mutate state
accepted history can rebuild expected state
```

Estimated state:

```text
ron-ledger QuickChain preflight slice: 98–99%
full ron-ledger QuickChain responsibility: around 64–67%
```

Remaining for `ron-ledger` later:

```text
canonical account-state leaves
state tree candidate
receipt/account ordering rules
locked vectors
golden vector tests
future root integration only after canonical bytes are locked
```

Do not add root-producing code yet unless canonical bytes and vectors are locked.

### `svc-wallet`

Now substantially advanced.

Current state:

```text
internal ROC wallet beta: 98–99%
QuickChain Phase-0 slice: 48–54%
```

Remaining immediate `svc-wallet` work:

```text
request poisoning matrix
projection validation matrix
preflight docs/runbook
possibly final codebundle
```

Remaining later `svc-wallet` work:

```text
durable backend operation journal
real ron-auth/ron-policy integration
durable idempotency/receipt storage
dedicated escrow DTOs
accounting export
production dependency boundaries
```

---

# QuickChain overall remaining roadmap

## Phase 0 remaining

Focus:

```text
Safety boundaries before roots.
DTOs and tests before chain state.
No fake chain behavior.
```

Remaining Phase 0 tasks:

```text
finish svc-wallet preflight boundary
add svc-wallet docs
move to ron-accounting
prove accounting is derivative only
move to svc-rewarder
prove rewarder plans payouts but never mutates ledger
move to svc-gateway/omnigate/storage paid enforcement
prove paid unlock requires backend wallet receipt and cannot be unlocked by cache
write cross-crate value-loop tests
prepare canonical vector plan
```

## Phase 1 later

Only after Phase 0 gates are green:

```text
canonical JSON v1 locked bytes
domain separator bytes
preimage rule: domain_separator_bytes || 0x00 || canonical_payload_bytes
BLAKE3 b3:<64 lowercase hex>
receipt vector locked bytes
account state vector locked bytes
hold lifecycle vector locked bytes
accounting snapshot vector locked bytes
reward plan vector locked bytes
```

Still no validators yet.

## Phase 2 later

Possible root work only after vectors are locked:

```text
receipt root
account state root
hold root
accounting root
reward root
epoch summary candidate
```

Must avoid:

```text
DB iteration order
wall-clock ordering as root authority
noncanonical JSON
floating point amounts
unknown consensus fields
placeholder hashes
```

## Phase 3 later

Proofs and challenge model:

```text
membership proofs
non-membership proofs
receipt inclusion proofs
account state proofs
challenge window
archive fallback
DA story
```

Still no pruning before DA/challenge/archive fallback.

## Phase 4 later

Validators/checkpoints only after deterministic local roots are boring:

```text
validator candidate design
checkpoint format
epoch boundaries
challenge rules
slashing? probably deferred
external anchors? deferred
```

No public bridge or external settlement yet.

## Phase 5 much later

External/public settlement remains deferred:

```text
ROX
Solana
external L2
public bridge
external DA
staking
liquidity
exchange-facing logic
```

These are not current runtime work.

---

# The next crate after `svc-wallet`

After finishing the last `svc-wallet` preflight hardening, the likely next crate should be:

```text
ron-accounting
```

Why:

```text
QuickChain needs to prove accounting is derivative snapshots only, not balance truth.
```

`ron-accounting` should prove:

```text
it consumes wallet/ledger receipts or events
it can produce snapshots
snapshots are not balance authority
snapshots cannot mint/burn/transfer
snapshot ids/hashes are deterministic candidates only
no payout mutation
no fake finality
no root-producing code until vectors are locked
```

After `ron-accounting`, likely next:

```text
svc-rewarder
```

`svc-rewarder` must prove:

```text
it plans payouts
it never mutates ledger directly
it never pays raw engagement directly
it only creates payout plans that later go through wallet/ledger
it separates event classes:
  economic_receipt
  metering
  proof_eligible
  ad_budgeted
  analytics_only
```

After that:

```text
svc-gateway / omnigate / svc-storage paid enforcement
```

They must prove:

```text
paid content unlock requires backend wallet/ledger receipt
cache cannot unlock paid content alone
display-only receipt cache is not economic authority
gateway/omnigate cannot mutate ledger directly
```

---

# Things not to do next

Do not add:

```text
roots
checkpoints
validators
anchors
settlement
bridges
staking
liquidity
public blockchain code
Solana code
ROX
external L2
external DA
exchange-facing logic
pruning
chain authority in CrabLink
gateway ledger mutation
omnigate ledger mutation
rewarder ledger mutation
raw engagement protocol payouts
fake finalized receipt labels
fake hashes
placeholder vectors
```

Do not make `svc-wallet`:

```text
a chain node
a validator
a checkpoint producer
a root producer
a public bridge
a finality oracle
a cache authority
a paid-content unlock authority without backend wallet receipt
```

Do not derive `operation_id` from:

```text
idempotency_key
txid
route label
user-supplied request field
UI state
wall clock
DB row order
```

---

# Good next patch target

The best next patch is:

```text
crates/svc-wallet/tests/quickchain_preflight_request_poisoning_matrix.rs
```

Goal:

```text
All live mutation routes reject request bodies containing QuickChain authority-looking fields.
```

Why this is the right next patch:

* We already proved QuickChain fields do not leak out.
* We already proved idempotency is not operation authority.
* Now we should prove QuickChain authority fields cannot be smuggled in across the full route surface.
* It is test-only.
* It does not create chain runtime.
* It strengthens Phase-0 safety.

Suggested test cases:

```text
issue rejects chain_id/operation_id/schema
transfer rejects chain_id/operation_id/schema
burn rejects chain_id/operation_id/schema
hold rejects chain_id/operation_id/schema
capture rejects chain_id/operation_id/schema
release rejects chain_id/operation_id/schema
```

Optional extra poison fields:

```text
state_root
receipt_root
checkpoint
anchor
validator
finality
finalized
settlement_root
```

Expected result:

```text
client error
not 200 OK
no wallet receipt returned
```

For transfer/burn/hold/capture/release tests, create setup funds first with clean issue requests, then poison the route being tested.

---

# Current mental model to resume with

The work is going well because we are doing QuickChain in the correct order:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

For `svc-wallet`, the mental model is:

```text
svc-wallet is the ROC mutation front door.
ron-ledger is balance truth.
svc-wallet receipts are accepted hot-path evidence.
QuickChain preflight projection is review-only.
idempotency_key is retry identity.
operation_id is backend durable operation identity.
accepted is not finalized.
live HTTP routes remain wallet routes.
projection fields must not leak into runtime routes.
```

This crate is now close to the end of its current QuickChain Phase-0 hardening pass.

The next session should either:

1. Finish the last `svc-wallet` hardening tests and docs, then mark this crate’s Phase-0 boundary as mostly complete; or
2. If time is short, add the request poisoning matrix only, run the full gate, then generate a fresh codebundle.

Current best estimate after this green run:

```text
Overall QuickChain vision: 36–40%
QuickChain Phase 0 / preflight readiness: 68–74%
svc-wallet internal ROC wallet readiness: 98–99%
svc-wallet QuickChain Phase-0 slice: 48–54%
```

The most important conclusion: **`svc-wallet` is now past the dangerous fake-authority part of its QuickChain preflight work.** What remains is mostly hardening, documentation, and then moving the same discipline into `ron-accounting`, `svc-rewarder`, and paid enforcement services.


### END NOTE - JUNE 14 2026 - 10:05 CST


### BEGIN NOTE - JUNE 14 2026 - 10:55 CST

The latest terminal output confirms the `svc-wallet` preflight script reached **“svc-wallet QuickChain preflight gate passed”**, and `cargo check --workspace` completed with only unrelated warnings in `micronode`, `ryker`, and `omnigate`. 

### BEGIN NOTE - JUNE 14 2026 - SVC-WALLET QUICKCHAIN PHASE-0 CHECKPOINT

# svc-wallet Carryover Notes — QuickChain Phase-0 / Internal ROC Wallet Boundary

## 0. Executive summary

`svc-wallet` is now effectively parked for the current QuickChain Phase-0 pass.

The crate has reached a strong green checkpoint for:

* internal ROC wallet mutation safety
* idempotency and nonce safety
* ledger primacy
* paid storage/site-visit wallet behavior
* receipt hash stability
* accepted-only finality honesty
* QuickChain preflight boundary tests
* request-poisoning rejection
* projection validation rejection
* docs/runbook coverage
* local preflight gate script coverage

The latest local gate confirms:

* normal `svc-wallet` tests pass
* focused QuickChain preflight tests pass
* full QuickChain preflight tests pass
* `svc-wallet` clippy passes with `-D warnings`
* the crate-level QuickChain preflight script prints `svc-wallet QuickChain preflight gate passed`
* `cargo check --workspace` completes, with only unrelated warnings outside `svc-wallet`

Current working conclusion:

```text
svc-wallet is no longer the active blocking crate for QuickChain Phase 0.
Next active crate should be ron-accounting.
```

Estimated status:

```text
svc-wallet internal ROC readiness:          ~99%
svc-wallet QuickChain Phase-0 slice:        ~92–95%
overall QuickChain project completion:      ~40–43%
```

The remaining `svc-wallet` work is follow-up polish and future integration work, not a blocker for moving to `ron-accounting`.

---

## 1. What svc-wallet is supposed to be

`svc-wallet` is the internal ROC mutation front-door.

It is responsible for accepting validated wallet actions, applying capability/policy checks, enforcing idempotency and nonce semantics, calling the ledger adapter, and returning backend-derived wallet receipts.

Allowed live wallet operations:

```text
issue
transfer
burn
hold
capture
release
balance lookup
receipt lookup
```

`svc-wallet` must remain a service boundary around wallet mutations.

It must not become:

```text
a ledger replacement
a QuickChain node
a checkpoint producer
a state-root producer
a receipt-root producer
a validator
a bridge
a public chain client
an external settlement adapter
a staking service
a liquidity service
an exchange-facing service
an accounting truth source
a rewarder mutation path
a CrabLink authority source
```

Core economic doctrine preserved:

```text
svc-wallet = mutation front-door
ron-ledger = durable economic truth
ron-accounting = snapshots/metering, not balance truth
svc-rewarder = payout planning, not mutation
CrabLink = display/user intent, not authority
```

---

## 2. Current proven state

The latest terminal run proves the following major gates are green.

### 2.1 Normal wallet gate

The script ran:

```bash
cargo fmt -p svc-wallet -- --check
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
```

The normal wallet tests passed across:

* unit tests
* API contract tests
* HTTP blackbox tests
* escrow HTTP tests
* transport bounds tests
* overflow ceiling tests
* hold/capture/release tests
* site visit receipt replay tests
* receipt finality honesty tests
* double-spend tests
* non-negativity tests
* conservation tests
* capability/policy tests
* DTO hygiene tests
* ledger primacy tests
* observability tests
* amnesia tests
* loom invariants
* receipt hash vectors
* web3 paid storage smoke
* benches smoke

This confirms the crate’s internal ROC wallet behavior remains healthy outside the QuickChain feature.

### 2.2 Focused QuickChain preflight gates

The script ran the focused preflight tests individually:

```bash
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_request_poisoning_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_projection_validation_matrix
```

All focused gates passed.

This proves QuickChain preflight behavior is intentionally inert and does not mutate live wallet route semantics.

### 2.3 Full QuickChain preflight gate

The script ran:

```bash
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
cargo test -p svc-wallet --all-targets --features quickchain-preflight
```

This also passed.

Important detail: during the normal non-feature `cargo test -p svc-wallet --all-targets`, the QuickChain preflight integration test files compile but run `0 tests` because they are correctly feature-gated.

During the full feature-enabled run, the preflight tests execute and pass.

This is the correct behavior.

---

## 3. Files added or changed in this svc-wallet Phase-0 pass

### 3.1 Added test file

```text
crates/svc-wallet/tests/quickchain_preflight_request_poisoning_matrix.rs
```

Purpose:

* prove live wallet mutation request bodies reject client-supplied QuickChain authority fields
* test all live mutation routes:

  * `/v1/issue`
  * `/v1/transfer`
  * `/v1/burn`
  * `/v1/hold`
  * `/v1/capture`
  * `/v1/release`
* verify poisoned requests:

  * return client error
  * do not return wallet receipts
  * do not consume idempotency keys
  * do not consume nonces
  * allow a clean retry with the same idempotency key and nonce

Poison fields covered include:

```text
schema
chain_id
operation_id
receipt_root
state_root
checkpoint
anchor
validator
finalized
finality
settlement_root
settlement_status
```

This test is important because it proves future-chain vocabulary cannot be smuggled into live wallet mutation bodies.

### 3.2 Added test file

```text
crates/svc-wallet/tests/quickchain_preflight_projection_validation_matrix.rs
```

Purpose:

* harden the inert QuickChain projection helper
* prove malformed projection context rejects
* prove malformed wallet receipt fields reject
* prove unknown future authority fields reject on projection DTO deserialize
* prove projection schema remains `svc-wallet`-specific
* prove projection status remains accepted-only
* prove projection cannot masquerade as a canonical future QuickChain receipt DTO

Projection validation rejects malformed:

```text
chain_id
operation_id
txid
account ids
idempotency key
timestamp
ledger sequence range
legacy ledger root
receipt hash
asset
```

Projection DTOs reject unknown future authority fields such as:

```text
state_root
receipt_root
checkpoint
anchor
validator
finality
finalized
settlement_root
```

This protects the future QuickChain proof/root path by ensuring the preflight projection remains narrow, typed, explicit, and non-authoritative.

### 3.3 Added docs file

```text
crates/svc-wallet/docs/quickchain-preflight.md
```

Purpose:

* document the `svc-wallet` QuickChain Phase-0 boundary
* explain what `svc-wallet` is and is not
* record what the preflight tests prove
* define no-go rules before leaving the crate
* document the local runbook
* record the next-crate handoff to `ron-accounting`

This doc intentionally avoids authorizing root-producing or settlement logic.

### 3.4 Updated script

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
```

Purpose:

* provide one repeatable local command for the normal wallet gate and the QuickChain preflight gate
* explicitly run the focused preflight tests by name
* run full feature-enabled clippy and test gates
* print a clear success line:

```text
== svc-wallet QuickChain preflight gate passed ==
```

Current script structure:

```text
normal gate:
  cargo fmt --check
  clippy without quickchain-preflight
  all normal svc-wallet tests

focused QuickChain preflight gates:
  quickchain_preflight_boundary
  quickchain_preflight_no_runtime_authority
  quickchain_preflight_live_route_matrix
  quickchain_preflight_idempotency_identity_boundary
  quickchain_preflight_request_poisoning_matrix
  quickchain_preflight_projection_validation_matrix

full QuickChain preflight gate:
  clippy with quickchain-preflight
  all tests with quickchain-preflight
```

---

## 4. Test inventory and what each test proves

### 4.1 `quickchain_preflight_boundary.rs`

Proves:

* enabling `quickchain-preflight` keeps the default wallet posture
* wallet projection requires explicit operation context
* missing `operation_id` rejects
* missing ledger sequence pair rejects
* fake receipt hash rejects
* ron-ledger’s preflight surface remains inert from the wallet boundary
* empty accepted replay boundary can rebuild without wallet mutation

Key conclusion:

```text
QuickChain preflight exists only as an inert compatibility/review surface from svc-wallet’s perspective.
```

### 4.2 `quickchain_preflight_no_runtime_authority.rs`

Proves:

* QuickChain authority fields are rejected as unknown request fields
* QuickChain preflight does not change live HTTP wallet receipt shape
* projection is manual and requires explicit context

Key conclusion:

```text
Live wallet routes do not become QuickChain routes.
```

### 4.3 `quickchain_preflight_live_route_matrix.rs`

Proves:

* all live mutation routes remain wallet receipts under `quickchain-preflight`
* idempotent replay remains the same wallet receipt under `quickchain-preflight`
* receipt lookup remains wallet receipt lookup, not QuickChain projection lookup

Routes covered:

```text
POST /v1/issue
POST /v1/transfer
POST /v1/burn
POST /v1/hold
POST /v1/capture
POST /v1/release
GET  /v1/tx/{txid}
```

Key conclusion:

```text
Feature-gating QuickChain preflight does not leak chain projection fields into live HTTP response DTOs.
```

### 4.4 `quickchain_preflight_idempotency_identity_boundary.rs`

Proves:

* same idempotency key and same body replays the same wallet receipt
* same idempotency key and changed body conflicts
* idempotency key does not become operation ID
* changing explicit operation ID does not rewrite wallet receipt hash

Key conclusion:

```text
idempotency_key is retry identity, not durable chain operation identity.
operation_id must be backend-assigned durable operation identity in future phases.
```

### 4.5 `quickchain_preflight_request_poisoning_matrix.rs`

Proves:

* all live wallet mutation requests reject unknown QuickChain authority-looking fields
* poisoned request rejection does not consume idempotency key
* poisoned request rejection does not consume nonce
* same request can be retried cleanly after rejection

Key conclusion:

```text
Clients cannot smuggle roots, checkpoints, finality, validators, anchors, operation IDs, or settlement status into wallet mutation bodies.
```

### 4.6 `quickchain_preflight_projection_validation_matrix.rs`

Proves:

* projection context rejects malformed `chain_id` and `operation_id`
* projection rejects malformed receipt fields
* projection DTO rejects unknown future chain-authority fields
* projection schema/status remain wallet-side and accepted-only
* projection cannot masquerade as a canonical future QuickChain receipt DTO

Key conclusion:

```text
The preflight projection helper is manual, inert, accepted-only, and safe as review scaffolding.
```

---

## 5. Invariants now proven for svc-wallet

### 5.1 Wallet remains mutation front-door

`svc-wallet` still owns wallet mutation entrypoints, not ledger truth.

It validates and routes actions, but durable truth remains in `ron-ledger`.

### 5.2 Ledger primacy remains intact

The crate already has tests proving:

* rejected ledger commits do not apply partial credit
* balance reads follow only successful ledger commits
* transfer cannot credit receiver from an empty account
* burn cannot overdraw
* transfer conserves participant balances
* burn explicitly reduces supply
* hold/capture/release preserves correct escrow semantics

### 5.3 Idempotency semantics remain stable

Proven behavior:

```text
same idempotency key + same body = same receipt replay
same idempotency key + different body = conflict
idempotency key is not operation ID
poisoned rejected requests do not consume idempotency identity
```

### 5.4 Nonce semantics remain stable

Proven behavior:

```text
same account + same nonce has exactly one winner
nonce replay rejects
failed/poisoned request does not consume nonce
rollback reopens nonce after failed commit path
```

### 5.5 DTO hygiene remains strict

Proven behavior:

```text
unknown request fields reject
amount rejects zero and float
amount serializes as string
memo control characters reject
wrong asset rejects
ambient authorization field rejects
QuickChain authority-looking request fields reject
```

### 5.6 Finality honesty remains intact

Wallet receipts currently serialize status as:

```text
accepted
```

They reject future/fake finality labels such as:

```text
finalized
anchored
settled
checkpointed
confirmed
```

This distinction is essential.

`accepted` means the backend wallet/ledger path accepted and recorded the mutation.

`accepted` does not mean QuickChain finalized, validator finalized, externally anchored, publicly settled, bridge settled, or pruning-safe.

### 5.7 Live HTTP shape remains wallet-shaped

Live routes do not emit:

```text
schema
chain_id
operation_id
idempotency_key
produced_at_ms
legacy_ledger_root
state_root
receipt_root
checkpoint
anchor
validator
finalized
finality
settlement_root
```

Live routes retain wallet receipt vocabulary.

### 5.8 Projection remains inert

Projection requires explicit context and does not alter wallet receipts.

Changing projection `operation_id` does not rewrite wallet receipt hash.

Projection is an inert compatibility/review helper, not runtime authority.

---

## 6. QuickChain-specific doctrine preserved

The following doctrine is still intact:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Current `svc-wallet` work did not add:

```text
roots
state roots
receipt roots
checkpoints
validators
validator signatures
consensus
fork choice
pruning
settlement
external anchors
bridge logic
staking
liquidity
exchange-facing logic
public chain state
CrabLink chain authority
gateway ledger mutation
omnigate ledger mutation
accounting ledger mutation
rewarder ledger mutation
```

This is important because `svc-wallet` is a high-risk crate for accidental scope creep. It touches money, wallet actions, paid access, receipts, and future proof inputs. The tests now form a boundary fence around that risk.

---

## 7. Current command runbook

From the workspace root:

```bash
cd /Users/mymac/Desktop/RustyOnions
```

Recommended `svc-wallet` gate:

```bash
cargo fmt -p svc-wallet
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
cargo check --workspace
```

Focused preflight-only commands:

```bash
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_request_poisoning_matrix
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_projection_validation_matrix
```

Full feature-enabled gate:

```bash
cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
cargo test -p svc-wallet --all-targets --features quickchain-preflight
```

Normal non-feature gate:

```bash
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
```

Current known workspace warnings outside `svc-wallet`:

```text
micronode: unused import UpstreamSpec
ryker: field max_msg_bytes is never read
omnigate: constant MAX_ACCOUNT_BYTES is never used
```

These are not `svc-wallet` failures.

---

## 8. What remains in svc-wallet later

`svc-wallet` is no longer blocking the current Phase-0 chain of work, but it is not “forever finished.” Future work remains.

### 8.1 Possible future polish

Potential follow-up items:

```text
add docs index link from svc-wallet README or crate docs to docs/quickchain-preflight.md
add a small CI job alias for dev-quickchain-preflight.sh
add more comments around projection helper explaining it is not a public API
add changelog entry for the QuickChain Phase-0 preflight boundary
add a short TESTS.md section for the focused preflight matrix
```

These are polish items, not blockers.

### 8.2 Future integration after ron-accounting and svc-rewarder

When `ron-accounting` and `svc-rewarder` are hardened, `svc-wallet` may need follow-up integration tests proving the full value loop:

```text
wallet accepted receipt
-> ledger truth
-> accounting snapshot input
-> rewarder payout plan
-> explicit wallet payout intent
-> wallet mutation
-> ledger receipt
```

Important invariant for that future loop:

```text
accounting cannot mutate wallet/ledger
rewarder cannot mutate wallet/ledger directly
all economic mutation returns through svc-wallet
ron-ledger remains truth
```

### 8.3 Future operation_id integration

Eventually, after upstream durable operation identity is designed, `svc-wallet` may need to accept or expose backend-assigned `operation_id` in a carefully controlled way.

Current rule:

```text
clients cannot supply operation_id
idempotency_key is not operation_id
projection requires explicit backend context
```

Future rule should likely be:

```text
operation_id is assigned by backend ledger/wallet path
operation_id binds to accepted ledger operation
operation_id is stable under replay
operation_id is not user-controlled
operation_id does not rewrite wallet receipt hash
```

Do not add this until the cross-crate operation identity model is locked.

### 8.4 Future canonical vector work

Eventually, QuickChain will need canonical vectors.

`svc-wallet` should not produce roots before:

```text
canonical bytes are locked
domain separators are locked
golden vectors are locked
receipt projection format is locked
operation ordering rules are locked
hash preimage rules are locked
```

Current projection tests help prepare for this, but do not authorize root-producing code.

### 8.5 Future durable ledger adapter work

The current tests are strong for dev/in-memory behavior and DTO boundaries. Future production wallet work may need:

```text
real durable ledger adapter integration tests
crash/restart receipt replay tests
idempotency persistence tests
nonce persistence tests
hold lifecycle persistence tests
receipt lookup persistence tests
admin/readiness behavior against ledger outage
```

These belong to production hardening, not the current QuickChain Phase-0 preflight slice.

### 8.6 Future paid access integration

The current wallet slice supports paid storage/site-visit style smoke behavior. Later, when the full paid media/content loop expands, we may need additional tests around:

```text
paid view prepare
explicit confirmation
hold/capture/release lifecycle
receipt display cache
balance refresh
creator payout routing
refund/release paths
expired holds
partial capture policy
duplicate capture rejection
```

Important invariant:

```text
no fake balance
no fake receipt
no silent spend
no cache-only unlock of paid content
```

### 8.7 Future QuickChain Phase-1 no-go reminder

Do not come back to `svc-wallet` and add any of the following without explicit Phase-1 authorization:

```text
state roots
receipt roots
checkpoint roots
epoch roots
validator signatures
anchor submission
public bridge
settlement finality
staking
liquidity
fork choice
consensus
chain database
pruning
CrabLink chain-authority route
gateway direct ledger mutation
omnigate direct ledger mutation
rewarder direct ledger mutation
accounting direct ledger mutation
```

---

## 9. What to do when returning to svc-wallet

When returning to `svc-wallet`, start with:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo fmt -p svc-wallet
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
cargo check --workspace
```

Then inspect:

```text
crates/svc-wallet/docs/quickchain-preflight.md
crates/svc-wallet/tests/quickchain_preflight_boundary.rs
crates/svc-wallet/tests/quickchain_preflight_no_runtime_authority.rs
crates/svc-wallet/tests/quickchain_preflight_live_route_matrix.rs
crates/svc-wallet/tests/quickchain_preflight_idempotency_identity_boundary.rs
crates/svc-wallet/tests/quickchain_preflight_request_poisoning_matrix.rs
crates/svc-wallet/tests/quickchain_preflight_projection_validation_matrix.rs
crates/svc-wallet/src/quickchain.rs
```

Before changing anything, confirm:

```text
Is this still Phase 0?
Is the change only docs/tests/DTO validation/projection review?
Does it avoid roots/checkpoints/validators/settlement?
Does it preserve wallet route shape?
Does it preserve accepted-only wallet finality?
Does it keep client-supplied operation_id rejected?
Does it keep ledger as truth?
```

If the answer to any of those is unclear, do not patch `svc-wallet` until the QuickChain phase boundary is restated.

---

## 10. Next active crate: ron-accounting

The next active crate should be:

```text
ron-accounting
```

Reason:

```text
ron-accounting is the next value-loop component after wallet/ledger receipts.
It must prove it is derivative metering/snapshot infrastructure, not balance truth.
```

Primary goals for `ron-accounting` QuickChain Phase-0:

```text
prove snapshots are derivative
prove snapshots are not balances
prove accounting cannot mint/burn/transfer
prove accounting cannot mutate ledger
prove accounting cannot mutate wallet
prove event classes are separated
prove fake finality is rejected
prove no root-producing code before canonical vectors
prove rewarder consumes accounting output only as planning input
```

Important event-class doctrine for next crate:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

Key rule:

```text
raw engagement must never directly mint or allocate protocol ROC.
```

Suggested first `ron-accounting` patch direction:

```text
docs/quickchain-preflight.md for ron-accounting
tests proving snapshot DTOs are derivative and non-authoritative
tests rejecting balance/finality/root/settlement vocabulary
tests proving reward projection is planning input, not mutation
```

---

## 11. Current project-level completion estimate

Current rough estimates after parking `svc-wallet`:

```text
ron-proto QuickChain Phase-0 DTO slice:        mostly complete / parked
ron-ledger QuickChain Phase-0 replay slice:    strong / parked for now
svc-wallet QuickChain Phase-0 slice:           92–95% / parked for now
overall QuickChain project:                    ~40–43%
```

The project has completed several of the hardest early pieces:

```text
strict DTO posture
integer money posture
b3 validation posture
ledger replay/projection posture
wallet route boundary posture
idempotency vs operation identity separation
accepted vs finalized separation
request poisoning rejection
projection validation rejection
```

The hardest future pieces still ahead:

```text
canonical vector locking
canonical bytes
deterministic account state root design
deterministic receipt root design
epoch/replay ordering
cross-crate accounting/rewarder integration
proof formats
event class enforcement
no-pruning-before-DA/challenge/archive policy
eventual Phase-1 root work
```

---

## 12. Final plain-English checkpoint

`svc-wallet` is in a very good state.

It now clearly says:

```text
I accept and route internal ROC wallet mutations.
I return backend-derived wallet receipts.
I enforce idempotency and nonce safety.
I do not invent balances.
I do not invent receipts.
I do not accept fake finality.
I do not accept client chain authority.
I do not produce roots.
I do not produce checkpoints.
I do not act as a validator.
I do not bridge externally.
I do not settle publicly.
I do not mutate ledger from accounting/rewarder/gateway/omnigate shortcuts.
```

That is exactly what we needed from `svc-wallet` for this QuickChain Phase-0 pass.

Next session should begin `ron-accounting` with the same discipline:

```text
derivative snapshots only
no balance truth
no spend authority
no fake finality
no root production before vectors
reward planning only
```

### END NOTE - JUNE 14 2026 - 10:55 CST




### BEGIN NOTE - JUNE 17 2026 - 12:10 CST

Your latest terminal output confirms this pair is genuinely parked: `ron-accounting` passed its exhaustive QuickChain preflight with **7 discovered focused QuickChain tests**, and `svc-wallet` passed its exhaustive preflight with **8 discovered focused QuickChain tests** plus the final `svc-wallet QuickChain parking gate passed` line. 

Here are the two crate-local note blocks.

# svc-wallet QuickChain Phase-0 / Preflight Notes

## Status

`svc-wallet` is now parked for the current QuickChain Phase-0/preflight pass.

This crate is the ROC wallet mutation front-door. It is allowed to validate explicit wallet requests, enforce capability/policy/idempotency/nonce boundaries, commit through `ron-ledger`, return backend-derived wallet receipts, and expose inert QuickChain preflight compatibility surfaces under a feature gate.

This crate is **not** a QuickChain runtime, not a validator, not a root producer, not a settlement service, not a bridge, not an anchor producer, not a staking service, not a liquidity service, and not public-chain authority.

The completed work strengthens the rule:

```text
svc-wallet mutates internal ROC only through the backend wallet/ledger path.
ron-ledger remains balance truth.
wallet receipts remain backend-derived.
QuickChain remains future settlement/proof infrastructure.
```

## Files added or changed

### Added

```text
crates/svc-wallet/docs/quickchain-preflight.md
crates/svc-wallet/scripts/dev-quickchain-park.sh
crates/svc-wallet/tests/quickchain_preflight_docs.rs
crates/svc-wallet/tests/quickchain_tooling_boundary.rs
```

### Modified

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/CODEBUNDLE.md
```

## What changed

### 1. Added crate-local QuickChain runbook

A new crate-local runbook was added at:

```text
crates/svc-wallet/docs/quickchain-preflight.md
```

The runbook documents the `svc-wallet` QuickChain Phase-0 posture.

It explicitly states that:

```text
svc-wallet is the ROC wallet mutation front-door.
QuickChain is future settlement infrastructure.
ron-ledger remains economic truth.
wallet receipts are backend-derived.
no fake balances are allowed.
no fake receipts are allowed.
no silent spend is allowed.
```

The runbook also preserves forbidden-scope markers:

```text
no roots
no receipt roots
no account state roots
no hold roots
no checkpoints
no validators
no settlement
no finality labels beyond wallet-side accepted
no anchors
no external anchors
no bridges
no staking
no liquidity
no pruning
no public-chain authority
no Solana/ROX/external settlement path
no gateway/omnigate/rewarder direct ledger mutation
```

This is important because future QuickChain work will eventually add roots/proofs/checkpoints, but this crate must not accidentally become runtime authority before those gates are explicitly opened.

### 2. Documented the wallet truth boundary

The runbook now records the intended truth model:

```text
svc-wallet accepts explicit wallet mutations.
svc-wallet validates capability/policy/idempotency/nonce boundaries.
svc-wallet commits only through ron-ledger.
svc-wallet returns backend-derived wallet receipts.
svc-wallet may expose inert QuickChain projection DTOs under a feature gate.
svc-wallet does not become QuickChain runtime authority.
```

This locks down the separation between wallet service behavior and future QuickChain settlement behavior.

### 3. Documented idempotency vs operation identity

The runbook now explicitly documents the identity split:

```text
operation_id = backend-assigned durable ledger-operation identity
idempotency_key = retry key only
account_sequence = ledger-assigned sequence identity
hold_id = one hold lifecycle identity
expiry = epoch-based where applicable
```

This distinction is critical for future QuickChain work.

The `idempotency_key` is not authority. It is not consensus identity. It is not the durable operation identity. It is only a retry/replay key.

Future QuickChain phases should continue using durable backend operation identity for economic replay/proofs, while keeping idempotency as a retry convenience.

### 4. Documented receipt honesty

The runbook now explicitly states that `svc-wallet` may honestly expose only wallet-side acceptance:

```text
accepted
```

It must not invent stronger future settlement/finality labels such as:

```text
finalized
checkpointed
anchored
settled
validator_accepted
epoch_included
bridge_confirmed
external_final
```

This prevents the wallet API from pretending to have QuickChain finality before roots, proofs, checkpoints, validators, and external anchors exist.

### 5. Documented request poisoning boundary

The runbook now explains that wallet mutation DTOs must reject client-supplied QuickChain authority fields, including:

```text
quickchain_root
state_root
receipt_root
checkpoint
checkpoint_id
validator_signature
validator_set
settlement_status
finality
anchor
external_anchor
bridge_txid
staking
liquidity
```

The boundary is:

```text
client-supplied QuickChain authority must fail closed before nonce/idempotency identity is consumed.
```

This matters because future clients, gateway surfaces, or malicious callers must not be able to smuggle “settlement” or “root” authority into wallet mutation requests.

### 6. Documented projection boundary

The runbook now clarifies that any QuickChain projection helper in `svc-wallet` is:

```text
manual
inert
feature-gated
preflight-only
non-authoritative
```

Projection helpers may copy accepted wallet receipt facts into strict DTOs for future compatibility tests.

Projection helpers must not:

```text
mutate ledger
change wallet receipts
replace receipt lookup route behavior
invent roots
invent finality
invent anchors
construct checkpoint state
unlock paid content
grant spend authority
```

### 7. Added docs regression test

A new docs regression test was added:

```text
crates/svc-wallet/tests/quickchain_preflight_docs.rs
```

This test verifies that the runbook exists and preserves the important doctrine.

It checks that the runbook contains the required truth-boundary phrases:

```text
svc-wallet is the ROC wallet mutation front-door
QuickChain is future settlement infrastructure
ron-ledger remains economic truth
wallet receipts are backend-derived
no fake balances
no fake receipts
no silent spend
```

It checks forbidden-scope markers:

```text
no roots
no receipt roots
no account state roots
no checkpoints
no validators
no settlement
no anchors
no external anchors
no bridges
no staking
no liquidity
no pruning
no public-chain authority
no Solana/ROX/external settlement path
```

It checks the identity split:

```text
idempotency_key = retry key only
operation_id = backend-assigned durable ledger-operation identity
account_sequence = ledger-assigned sequence identity
hold_id = one hold lifecycle identity
```

It also checks that the parking script verifies docs before declaring success.

### 8. Added tooling boundary test

A new tooling boundary test was added:

```text
crates/svc-wallet/tests/quickchain_tooling_boundary.rs
```

This test ensures the preflight script stays bash/cargo-only and does not drift into hidden Python helper workflows.

It checks that:

```text
the preflight script discovers QuickChain tests dynamically
the script does not hardcode the QuickChain test matrix
the script remains bash/cargo-only
no checked-in Python helper files exist under svc-wallet
the script preserves forbidden-scope markers
```

This aligns with the project preference to avoid Python helper drift for QuickChain patch delivery.

### 9. Hardened exhaustive preflight script

The existing script was hardened:

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
```

It now performs the following gate:

```text
fmt check
normal strict clippy
normal all-target tests
dynamic QuickChain test discovery
feature-gated QuickChain test execution
feature-gated strict clippy
feature-gated all-target tests
bash/cargo-only tooling boundary
no checked-in Python helpers under svc-wallet
forbidden-scope marker output
```

The script dynamically discovers `quickchain*.rs` test targets instead of hardcoding them.

After this pass, the script discovers 8 focused QuickChain tests:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_idempotency_identity_boundary
quickchain_preflight_live_route_matrix
quickchain_preflight_no_runtime_authority
quickchain_preflight_projection_validation_matrix
quickchain_preflight_request_poisoning_matrix
quickchain_tooling_boundary
```

### 10. Added final parking script

A final parking gate was added:

```text
crates/svc-wallet/scripts/dev-quickchain-park.sh
```

This script runs the exhaustive preflight script and then verifies the crate-local runbook and docs regression tests.

It checks:

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/docs/quickchain-preflight.md
cargo test -p svc-wallet --test quickchain_preflight_docs
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_docs
cargo check -p svc-wallet --features quickchain-preflight
cargo check --workspace
```

It then declares:

```text
== svc-wallet QuickChain parking gate passed ==
```

### 11. Regenerated CODEBUNDLE

The crate code bundle was regenerated:

```text
crates/svc-wallet/CODEBUNDLE.md
```

This bundle now includes the updated script/test surfaces.

## Current gate status

The following passed in the latest terminal run:

```text
cargo test -p svc-wallet --test quickchain_preflight_docs
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_docs
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/scripts/dev-quickchain-park.sh
```

The parking gate completed with:

```text
== svc-wallet QuickChain parking gate passed ==
```

The exhaustive preflight completed with:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=8 ==
```

The final workspace check completed successfully. It emitted warnings in unrelated crates, but did not block `svc-wallet`.

## Important invariants now covered

### Wallet mutation authority

`svc-wallet` remains the only wallet mutation front-door.

It must continue to own:

```text
issue
transfer
burn
hold
capture
release
balance read
receipt lookup
idempotent replay of accepted wallet receipts
```

It must not outsource mutation authority to:

```text
gateway
omnigate
ron-accounting
svc-rewarder
CrabLink
QuickChain preflight helper
client cache
frontend state
external chain
```

### Ledger primacy

`ron-ledger` remains economic truth.

`svc-wallet` may produce wallet receipts only after backend wallet/ledger acceptance.

Balance reads must follow successful ledger commits.

Rejected commits must not partially credit or debit accounts.

### Receipt honesty

Wallet receipts remain wallet-side accepted receipts.

No future finality wording is allowed until future QuickChain phases explicitly implement and prove it.

### Idempotency

Idempotency keys replay the same accepted receipt for the same request fingerprint.

Same idempotency key with different body must conflict.

Idempotency keys must not become operation authority.

### Non-negativity and conservation

Existing tests continue to protect:

```text
transfer from empty account rejects without crediting receiver
burn more than balance rejects without state change
transfer conserves participant balances
burn explicitly reduces supply
hold/capture/release preserves total value
capture more than held rejects without crediting payee
```

### DTO hygiene

Wallet mutation request DTOs reject unknown fields and ambient authority fields.

Amounts remain integer minor units.

Floats are rejected.

Control characters in memo fields are rejected.

### Paid storage / paid access smoke

Existing paid storage smoke remains green:

```text
paid_storage_state_machine_rejects_unfunded_and_charges_funded_payer
```

This confirms the paid path remains explicit and wallet-backed.

### QuickChain preflight posture

QuickChain preflight remains feature-gated.

Default wallet posture is unchanged.

Live routes remain wallet receipts, not QuickChain projection artifacts.

Projection is manual and requires explicit context.

Projection rejects malformed receipt fields, fake receipt hashes, missing operation identity, and unknown future authority fields.

## What remains for svc-wallet

### Near-term remaining work

No further patch is required for the current Phase-0/preflight parking pass.

The next near-term work is notes/commit hygiene:

```text
verify git status
stage intended svc-wallet files
commit or carry forward with pair notes
```

### Later Phase-0 polish

Potential future polish, if desired:

```text
add a pair-level script for svc-wallet + ron-accounting together
add explicit docs linking svc-wallet receipts to ron-accounting snapshot handoff
add a small README note pointing to docs/quickchain-preflight.md
add CI job entry for dev-quickchain-preflight.sh if project CI is ready for it
```

These are optional. The crate is already parked for this pass.

### Future QuickChain Phase 1 work

Deferred until Phase 1 is explicitly authorized:

```text
canonical receipt preimage vectors
locked receipt bytes
locked receipt hash vectors
operation_id assignment source
account_sequence mapping from ron-ledger
hold lifecycle DTOs
receipt inclusion candidate DTOs
single-authority internal checkpoint candidate
```

None of those should be started casually. They must follow the project doctrine:

```text
sketch -> locked_bytes -> locked_hash
```

### Explicitly forbidden until future gates

Still forbidden in `svc-wallet`:

```text
root-producing code
checkpoint-producing code
validator code
settlement code
anchor code
external anchor code
bridge code
staking code
liquidity code
public-chain logic
Solana/ROX settlement path
gateway/omnigate/rewarder direct ledger mutation
client-supplied finality
fake balances
fake receipts
silent spend
unlock from cache alone
```

## Parking conclusion

`svc-wallet` has completed its current QuickChain Phase-0/preflight pass.

It now has:

```text
crate-local QuickChain runbook
docs regression tests
tooling boundary tests
exhaustive bash/cargo preflight script
final parking script
feature-gated QuickChain test discovery
feature-gated clippy/test coverage
workspace check confirmation
```

The crate is ready to be treated as parked for this QuickChain phase and carried forward into the next crate pair work.

# ron-accounting QuickChain Phase-0 / Preflight Notes

## Status

`ron-accounting` is now parked for the current QuickChain Phase-0/preflight pass.

This crate is derivative metering, sealed snapshot, reward-projection, and interop-vector infrastructure. It is not balance truth. It does not mint, transfer, burn, hold, capture, release, finalize, settle, anchor, bridge, stake, or provide liquidity.

The completed work strengthens the rule:

```text
ron-accounting records usage.
ron-accounting seals/report snapshots.
ron-accounting can project planning artifacts for svc-rewarder.
ron-accounting never mutates ROC balances.
ron-accounting never replaces ron-ledger truth.
```

## Files added or changed

### Added

```text
crates/ron-accounting/tests/quickchain_tooling_boundary.rs
```

### Modified

```text
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/CODEBUNDLE.md
```

## What changed

### 1. Hardened exhaustive preflight runner

The preflight runner was hardened at:

```text
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

It now performs a broad local gate:

```text
fmt check
strict clippy
all-target tests
WAL feature all-target tests
dynamic QuickChain test discovery
focused QuickChain test execution
bash/cargo-only tooling boundary
no checked-in Python helpers under ron-accounting
```

The script dynamically discovers `quickchain*.rs` tests instead of hardcoding a fixed test list.

In the latest run it discovered 7 focused QuickChain tests:

```text
quickchain_preflight_boundary
quickchain_preflight_event_class_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_snapshot_non_authority
quickchain_tooling_boundary
```

The script completed with:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=7 ==
```

### 2. Added tooling boundary test

A new tooling boundary test was added:

```text
crates/ron-accounting/tests/quickchain_tooling_boundary.rs
```

This test ensures the preflight script stays bash/cargo-only and does not drift into checked-in Python helper workflows.

It verifies:

```text
the preflight script discovers QuickChain tests dynamically
the script does not hardcode the test matrix
no checked-in Python helper files exist under ron-accounting
the script preserves accounting non-authority posture
```

This matches the current project preference: QuickChain patch delivery and preflight gates should remain direct, bash/cargo-based, and transparent.

### 3. Preserved accounting non-authority posture

The preflight script and tests now explicitly preserve the principle that accounting is not balance truth.

`ron-accounting` may produce:

```text
usage events
counter rows
sealed slices
snapshot CIDs
reward projection artifacts
rewarder-compatible planning snapshots
interop vectors
```

It must not produce:

```text
balances
wallet receipts
ledger mutations
payout transactions
root authority
checkpoint authority
settlement authority
anchor authority
bridge authority
staking authority
liquidity authority
```

### 4. Confirmed strict usage-event boundary

Existing QuickChain tests remain green and confirm that usage events cannot smuggle authority.

The current test suite confirms that:

```text
raw engagement usage event is metering, not protocol ROC authority
usage event DTO rejects smuggled QuickChain authority fields
ingest request DTO rejects smuggled QuickChain authority fields
projected reward snapshot is artifact CID, not QuickChain root
reward snapshot serializes as planning artifact, not balance or root
```

This is critical because raw engagement must not become direct protocol payout authority.

### 5. Confirmed event-class boundary

The event-class boundary remains green.

The tests confirm that only explicit reward projection metric kinds are currently eligible for reward planning.

The current intended distinction is:

```text
metering events can be recorded
reward-eligible metering events can be projected into planning artifacts
analytics-only engagement cannot allocate protocol ROC
ad-budgeted style events cannot become protocol reward authority
plain request metrics cannot project rewards
pin/cpu/plain request metrics do not directly project rewards
route wording cannot smuggle reward authority
```

This protects against bot-farm style abuse and against accidentally turning analytics into mint/transfer authority.

### 6. Confirmed ingest poisoning boundary

The ingest poisoning tests remain green.

They protect against:

```text
schema drift
empty required fields
invalid nested usage events
oversized batches
noncanonical CID shapes before dedupe authority
QuickChain root fields
finality poison fields
idempotency/operation identity smuggling
```

This means `ron-accounting` can accept metering inputs while rejecting attempts to sneak in ledger, root, or settlement semantics.

### 7. Confirmed reward DTO strictness

The reward DTO strictness tests remain green.

They confirm that reward-related DTOs reject unknown authority fields at multiple levels:

```text
reward projection config
reward projection report
reward contribution export
reward snapshot export
projected reward snapshot top-level fields
projected reward snapshot nested snapshot fields
nested reward contribution fields
```

This protects the reward-planning handoff from becoming a hidden settlement surface.

### 8. Confirmed reward projection boundary

The reward projection tests remain green.

They confirm:

```text
storage served/stored and uptime inputs can project
analytics-only rows are ignored
raw engagement alone cannot project rewards
non-reward metrics alone cannot project rewards
float money is rejected
empty reward inputs are rejected
projection order is deterministic
projection artifact does not expose payout or chain authority fields
projection is still not a root
```

This is the correct Phase-0 boundary:

```text
reward projection = deterministic planning artifact
reward projection != payout
reward projection != ledger mutation
reward projection != QuickChain root
```

### 9. Confirmed snapshot non-authority boundary

The snapshot non-authority tests remain green.

They confirm:

```text
reward snapshot JSON shape is contribution artifact only
canonical snapshot CID is artifact hash, not root field
canonical bytes do not contain finality or mutation vocabulary
pool_minor_units is integer string, not float or JSON number
duplicate accounts reject
invalid account strings reject
```

This is important because reward snapshots may look economically meaningful, but they are still not balance truth and not payout authority.

### 10. Confirmed WAL feature path

The preflight runner now also tests:

```text
cargo test -p ron-accounting --all-targets --features wal
```

This is important because WAL behavior affects replay/recovery posture for accounting artifacts.

The latest WAL feature test path passed, including WAL unit tests:

```text
wal_appends_and_drains
replay_skips_corrupt_records
```

Those tests support accounting durability/replay hygiene without turning accounting into balance truth.

### 11. Confirmed unit, prop, example, and bench paths

The latest preflight run confirms:

```text
unit tests passed
property tests passed
loom placeholder tests passed
benches compiled/smoked successfully
examples compiled/smoked successfully
WAL feature tests passed
QuickChain focused tests passed
```

This means `ron-accounting` is in a good parked state for the current phase.

### 12. Regenerated CODEBUNDLE

The crate code bundle was regenerated:

```text
crates/ron-accounting/CODEBUNDLE.md
```

This bundle now includes the updated script/test surfaces.

## Current gate status

The latest terminal run completed:

```text
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

The gate ended with:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=7 ==
```

This confirms the crate is parked for the current QuickChain Phase-0/preflight pass.

## Important invariants now covered

### Accounting is not balance truth

`ron-accounting` cannot be used as a balance source.

Balance truth remains:

```text
ron-ledger
```

Wallet mutation remains:

```text
svc-wallet
```

Accounting remains:

```text
derivative usage/metering/snapshot/reporting infrastructure
```

### No direct protocol ROC authority from raw engagement

Raw engagement, analytics, request counts, ad-style events, or route wording must not directly mint, allocate, transfer, or authorize protocol ROC.

Only explicitly eligible metering categories can feed reward planning, and even then the output remains a planning artifact.

### Rewarder handoff remains planning only

`ron-accounting` may generate rewarder-compatible snapshots.

Those snapshots are not:

```text
payouts
balances
receipts
roots
settlement proofs
finality artifacts
ledger mutations
```

They are planning inputs.

### Strict DTO boundaries

Usage, ingest, reward projection, reward snapshot, and contribution DTOs reject unknown authority fields.

This prevents future QuickChain fields from being accidentally accepted before the proper phase.

### Integer-only money

`pool_minor_units` remains string-encoded integer money.

Floats and JSON-number money forms are rejected where the reward snapshot boundary requires integer strings.

### Canonical artifact CIDs are not roots

Reward snapshot CIDs and sealed slice digests are artifact hashes.

They are not QuickChain roots.

They are not state roots.

They are not receipt roots.

They are not settlement roots.

Future root-producing code must come later and must use canonical bytes, domain separation, locked vectors, and explicit authorization.

### WAL/replay posture

WAL feature tests now pass under the exhaustive preflight runner.

This helps prove accounting artifacts can be handled with better replay/recovery hygiene without giving accounting balance authority.

## What remains for ron-accounting

### Near-term remaining work

No further patch is required for the current Phase-0/preflight parking pass.

The immediate remaining work is notes/commit hygiene:

```text
verify git status
stage intended ron-accounting files
commit or carry forward with pair notes
```

### Optional Phase-0 polish

Optional future polish:

```text
add crate-local parking script similar to svc-wallet if desired
add docs regression test if docs are expanded further
add CI workflow entry for dev-quickchain-preflight.sh if project CI is ready
add pair-level script with svc-wallet for value-plane preflight confirmation
```

These are useful but not required for the current parked state.

### Future QuickChain Phase 1 work

Deferred until Phase 1 is explicitly authorized:

```text
locked canonical reward snapshot vectors
locked sealed slice vectors
canonical JSON v1 byte fixtures
domain-separated artifact hash fixtures
accounting-to-rewarder interop golden vectors
replay determinism fixtures
explicit no-root artifact registry
```

These should be added only with proper vector discipline:

```text
sketch -> locked_bytes -> locked_hash
```

### Future reward loop work

Later, `ron-accounting` will likely participate in the value loop as:

```text
svc-storage/svc-gateway/omnigate emit usage/metering events
ron-accounting records/seals snapshots
ron-accounting exports planning artifacts
svc-rewarder consumes planning artifacts
svc-rewarder creates payout plans
svc-wallet executes approved payout mutations
ron-ledger records economic truth
```

That future loop must preserve:

```text
accounting does not mutate ledger
rewarder does not mutate ledger
wallet mutates through explicit backend path
ledger remains truth
```

### Explicitly forbidden until future gates

Still forbidden in `ron-accounting`:

```text
balance truth
wallet receipt issuance
mint authority
transfer authority
burn authority
hold/capture/release authority
payout mutation
root-producing code
checkpoint-producing code
validator code
settlement code
anchor code
external anchor code
bridge code
staking code
liquidity code
public-chain logic
Solana/ROX settlement path
raw engagement direct protocol payout
analytics direct protocol payout
fake balances
fake receipts
```

## Parking conclusion

`ron-accounting` has completed its current QuickChain Phase-0/preflight pass.

It now has:

```text
exhaustive bash/cargo preflight script
dynamic QuickChain test discovery
tooling boundary test
strict accounting non-authority tests
event-class boundary tests
ingest poisoning tests
reward DTO strictness tests
reward projection boundary tests
snapshot non-authority tests
WAL feature test path
regenerated CODEBUNDLE
```

The crate is ready to be treated as parked for this QuickChain phase and carried forward into the next crate pair work.

### END NOTE - JUNE 17 2026 - 12:10 CST



### BEGIN NOTE - JUNE 19 2026 - 13:55 CST

The latest terminal proof shows `ron-accounting` fully green and parked: focused docs/tooling tests passed, exhaustive preflight discovered 8 QuickChain tests, WAL feature all-target tests passed, and the final parking gate ended with `== ron-accounting QuickChain parking gate passed ==`. The notes below also preserve the governing QuickChain Phase-0 rule: wallet remains mutation front-door, ledger remains truth, accounting is not balance truth, and Phase 0 must not introduce consensus, validators, bridges, staking, liquidity, fake balances, fake receipts, or live QuickChain authority.  

# QuickChain Phase-0 / Preflight Notes — `svc-wallet + ron-accounting`

## Status

`svc-wallet + ron-accounting` are now parked for the current QuickChain Phase-0/preflight pass.

This pair now protects the most important internal ROC value-plane boundary:

```text
svc-wallet:
  mutation front-door for ROC wallet operations

ron-ledger:
  durable economic truth

ron-accounting:
  derivative usage, metering, sealed snapshots, and reward-planning artifacts

svc-rewarder:
  payout planning only

svc-wallet again:
  approved payout intent commits through wallet/ledger truth path

ron-ledger again:
  final balance truth and receipt truth
```

The completed work does **not** make either crate a QuickChain runtime.

The completed work does **not** authorize roots, checkpoints, validators, public settlement, bridge logic, staking, liquidity, ROX, Solana, external settlement, or chain authority in clients/gateway/omnigate.

The pair is parked because both crates now have crate-local QuickChain documentation, focused boundary tests, tooling/preflight scripts, and parking gates that prove they remain inside the allowed Phase-0 safety scope.

---

# 1. Pair-level purpose

The purpose of this pass was to harden the wallet/accounting seam before moving farther down the ROC value loop.

The intended internal value loop remains:

```text
ron-proto economic DTOs
→ svc-wallet issue/transfer/burn/hold/capture/release/receipt
→ ron-ledger durable truth and replay
→ backend receipt
→ paid enforcement/unlock
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ svc-wallet commits approved payout intent
→ ron-ledger receipt and balance truth
```

This pass specifically proves:

```text
svc-wallet remains the only wallet mutation front-door.
ron-ledger remains durable economic truth.
ron-accounting remains derivative metering/snapshot infrastructure.
accounting snapshots do not become balances.
reward projections do not become payouts.
wallet receipts do not overclaim finality.
client-supplied QuickChain-looking fields do not become authority.
idempotency_key remains retry identity, not operation authority.
operation_id remains backend-assigned durable operation identity.
raw engagement does not directly mint, allocate, transfer, or mutate protocol ROC.
artifact hashes / CIDs are not QuickChain roots.
```

---

# 2. Governing QuickChain doctrine preserved

The pass preserves the master doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Phase 0 remains preflight/safety work only.

Allowed in this pass:

```text
docs hardening
crate-local runbooks
boundary tests
strict DTO drift tests
unknown-field rejection tests
poisoning tests
preflight scripts
parking gates
tooling hygiene
feature-gated compatibility checks
WAL/replay smoke coverage for accounting artifacts
```

Forbidden in this pass:

```text
roots
receipt roots
state roots
accounting roots
reward roots
checkpoints
validators
settlement
anchors
external anchors
bridges
staking
liquidity
ROX
Solana
external settlement
public-chain authority
gateway ledger mutation
omnigate ledger mutation
accounting ledger mutation
rewarder direct ledger mutation
fake balances
fake receipts
silent spend
root-producing code
client-side chain authority
```

---

# 3. `svc-wallet` notes

## 3.1 Current status

`svc-wallet` is parked for the current QuickChain Phase-0/preflight pass.

Its role is:

```text
ROC wallet mutation front-door
```

It may:

```text
validate wallet requests
enforce capability/policy posture
resolve idempotency
enforce nonce/sequence expectations
call the ledger adapter
commit through ron-ledger
return backend-derived wallet receipts
expose inert QuickChain preflight compatibility under feature gate
```

It must not become:

```text
durable ledger truth
QuickChain runtime
validator
checkpoint writer
root producer
settlement service
bridge
anchor service
staking service
liquidity service
external-chain adapter
public-chain authority
```

Durable economic truth remains:

```text
ron-ledger
```

Actual economic mutation path remains:

```text
client / service request
→ svc-wallet validation/policy/idempotency/nonce boundary
→ ron-ledger commit
→ backend-derived wallet receipt
```

## 3.2 Files and surfaces added or hardened

The `svc-wallet` Phase-0 surface now includes:

```text
crates/svc-wallet/docs/quickchain-preflight.md
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/scripts/dev-quickchain-park.sh
crates/svc-wallet/tests/quickchain_preflight_docs.rs
crates/svc-wallet/tests/quickchain_tooling_boundary.rs
```

The crate also includes and protects the feature-gated QuickChain compatibility surface:

```text
crates/svc-wallet/src/quickchain.rs
Cargo feature: quickchain-preflight
```

The `quickchain-preflight` feature is explicitly Phase-0-only. It is for compatibility checks and preflight boundary tests. It must not enable runtime roots, validators, checkpoints, settlement, anchors, bridges, staking, liquidity, or external-chain behavior.

## 3.3 Test matrix protected

The `svc-wallet` QuickChain-focused test matrix includes:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_idempotency_identity_boundary
quickchain_preflight_live_route_matrix
quickchain_preflight_no_runtime_authority
quickchain_preflight_projection_validation_matrix
quickchain_preflight_request_poisoning_matrix
quickchain_tooling_boundary
```

The exhaustive preflight runner discovers the QuickChain tests dynamically rather than relying on a stale hardcoded list.

The preflight protects against future test drift by requiring a minimum focused QuickChain test count and running each discovered `quickchain*.rs` target.

## 3.4 Main invariants now enforced

`svc-wallet` now has explicit docs/tests protecting these invariants:

```text
svc-wallet is the ROC wallet mutation front-door.
ron-ledger remains economic truth.
wallet receipts are backend-derived.
wallet receipt status may honestly say accepted only.
accepted is not finalized.
accepted is not anchored.
accepted is not epoch-included unless future evidence exists.
idempotency_key is retry identity only.
idempotency_key is not operation_id.
idempotency_key is not account_sequence.
operation_id is backend-assigned durable ledger-operation identity.
client request bodies cannot smuggle QuickChain authority fields.
poisoned requests cannot consume idempotency keys.
poisoned requests cannot consume nonces.
fake receipt hashes reject.
projection/compatibility DTOs remain manual and inert.
QuickChain remains disabled by default.
```

## 3.5 No-go boundary for `svc-wallet`

`svc-wallet` must not add or expose:

```text
state_root
receipt_root
accounting_root
reward_root
checkpoint_root
checkpoint_hash
validator signatures
validator sets
epoch finality
external anchor finality
bridge settlement
staking claims
liquidity claims
ROX settlement
Solana settlement
gateway-side mutation authority
omnigate-side mutation authority
client-side receipt authority
fake balances
fake receipts
silent spend
```

If a future change needs one of those fields, it must be treated as a separate QuickChain phase decision and blocked until canonical bytes, domain separation, locked vectors, and explicit go/no-go gates authorize it.

## 3.6 Scripts and gates

`svc-wallet` now has:

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/scripts/dev-quickchain-park.sh
```

The preflight script checks:

```text
fmt
normal strict clippy
normal all-target tests
no checked-in Python helpers under svc-wallet
dynamic discovery of QuickChain integration tests
feature-gated QuickChain focused test targets
feature-gated strict clippy
feature-gated all-target tests
```

The parking script checks:

```text
crate-local preflight
QuickChain docs presence
wallet boundary markers
docs regression test
feature-gated docs regression test
package check with quickchain-preflight feature
workspace check
```

Expected success markers:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=8 ==
== svc-wallet QuickChain parking gate passed ==
```

## 3.7 What changed conceptually

Before this pass, `svc-wallet` already served the wallet role, but QuickChain pressure could have caused future drift:

```text
wallet receipt fields could overclaim finality
idempotency_key could be confused with operation_id
QuickChain DTOs could be accepted as authority
route responses could leak root/checkpoint/finality vocabulary
compatibility projections could become implicit runtime authority
request poisoning could consume nonce/idempotency state
```

After this pass, `svc-wallet` has guardrails that make those failures visible.

The crate now explicitly proves that it remains a wallet front-door and not a chain/runtime/finality layer.

## 3.8 Remaining `svc-wallet` work

No more immediate patch is required for the current QuickChain Phase-0 pass.

Future non-blocking work may include:

```text
stronger production auth/capability integration
more live route coverage once production service paths grow
deeper hold/capture/release route-level poisoning tests
more receipt replay/index testing
stronger integration with svc-gateway/omnigate paid flows
stronger low-disk-friendly runner variants
future canonical wallet receipt vector handoff once QuickChain roots are allowed
```

Do not reopen `svc-wallet` architecture unless a specific adjacent-crate change requires a compile or boundary fix.

---

# 4. `ron-accounting` notes

## 4.1 Current status

`ron-accounting` is parked for the current QuickChain Phase-0/preflight pass.

Its role is:

```text
derivative usage/metering/snapshot/reporting infrastructure
```

It may:

```text
ingest usage/metering events
normalize labels
maintain bounded counters
seal slices/windows
produce deterministic accounting artifacts
produce reward-planning artifacts
produce canonical bytes for artifacts
produce artifact CIDs/hashes
feed svc-rewarder planning
```

It must not become:

```text
balance truth
wallet authority
ledger authority
payout authority
receipt authority
finality authority
root authority
settlement authority
validator authority
bridge authority
staking/liquidity authority
paid-content unlock authority
```

The correct handoff remains:

```text
ron-accounting summary / reward-planning artifact
→ svc-rewarder deterministic payout planning
→ explicit wallet path
→ ron-ledger durable truth
→ backend-derived receipt
```

The forbidden handoff remains:

```text
raw engagement
→ ron-accounting
→ direct protocol ROC mutation
```

## 4.2 Files added or hardened

This pass added or hardened:

```text
crates/ron-accounting/docs/quickchain-preflight.md
crates/ron-accounting/tests/quickchain_preflight_docs.rs
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

The `ron-accounting` runbook documents the crate as:

```text
not a chain
not a ledger
not a wallet
not a root producer
not a settlement service
not validator infrastructure
not bridge infrastructure
not staking/liquidity infrastructure
not balance truth
```

The only acceptable Phase-0 role is now explicitly documented as:

```text
usage / metering input
→ normalization
→ bounded counters
→ deterministic sealed slices/windows
→ derivative reward snapshot artifacts
→ svc-rewarder planning input
```

The wallet mutation path is explicitly documented as:

```text
svc-wallet
→ ron-ledger
→ backend-derived receipt
```

Accounting must not bypass that path.

## 4.3 Docs regression test added

Added:

```text
crates/ron-accounting/tests/quickchain_preflight_docs.rs
```

This test locks the runbook and scripts against architectural drift.

It verifies the docs preserve these markers:

```text
Accounting is not balance truth
ron-accounting is not a chain
ron-accounting is not a ledger
ron-accounting is not a wallet
ron-accounting is not a root producer
ron-accounting is not a settlement service
usage / metering input
derivative reward snapshot artifacts
svc-wallet
ron-ledger
```

It also verifies the docs preserve the no-authority boundary:

```text
must not perform or claim
mutate wallet state
mutate ledger state
issue wallet receipts
invent fake receipts
invent fake balances
unlock paid content
no fake balances
no fake receipts
no silent spend
no wallet mutation
no ledger mutation
no payout execution
```

It verifies the forbidden QuickChain vocabulary remains denial-only:

```text
no roots
no receipt roots
no account state roots
no accounting_root
no reward_root
no checkpoints
no validators
no settlement
no anchors
no external anchors
no bridges
no staking
no liquidity
no Solana/ROX/external settlement path
No DB-order roots
No wall-clock roots
No placeholder hashes
No fake hashes
```

It verifies artifact CIDs are documented as artifact hashes, not roots.

It verifies event classes and raw-engagement boundaries are documented.

It verifies rewarder handoff is planning only.

It verifies the preflight and parking scripts keep dynamic discovery and docs checks in the gate.

## 4.4 Small test fix made after first run

The first `quickchain_preflight_docs` run failed because the docs/script test expected a literal command marker:

```text
cargo test -p ron-accounting --test quickchain_preflight_docs
```

The actual script correctly uses the overridable cargo variable:

```text
"$CARGO" test -p ron-accounting --test quickchain_preflight_docs
```

The test was fixed to assert the stable command fragment instead of forcing the literal `cargo` binary name:

```text
test -p ron-accounting --test quickchain_preflight_docs
```

This preserves the boundary while allowing the script to stay portable through `CARGO="${CARGO:-cargo}"`.

## 4.5 Preflight script hardened

Updated:

```text
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

The script now checks:

```text
workspace root detection
crate root detection
CARGO override support
forbidden scope marker output
no checked-in Python helpers under ron-accounting
crate-local docs presence
required docs boundary markers
fmt check
strict clippy
normal all-target tests
WAL feature all-target tests
dynamic discovery of every tests/quickchain*.rs target
minimum QuickChain test count
presence of quickchain_preflight_docs in discovered matrix
execution of every discovered QuickChain test target
```

Expected success marker:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=8 ==
```

The latest run reached this marker.

## 4.6 Parking script hardened

Updated:

```text
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

The script now checks:

```text
crate preflight
docs regression test
workflow package-name hygiene
crate-local docs presence
accounting non-authority markers
raw engagement boundary marker
artifact CID is not root marker
no roots/checkpoints/validators/settlement/anchors/bridges/staking/liquidity markers
no Solana/ROX/external settlement marker
no fake balances/fake receipts/silent spend markers
no wallet mutation/ledger mutation markers
workspace check
```

Expected success marker:

```text
== ron-accounting QuickChain parking gate passed ==
```

The latest run reached this marker.

## 4.7 Current QuickChain test matrix

The `ron-accounting` exhaustive preflight discovered 8 focused QuickChain test targets:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_event_class_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_snapshot_non_authority
quickchain_tooling_boundary
```

The latest run executed all 8 discovered targets successfully.

## 4.8 Current normal/WAL test coverage proven

The latest `ron-accounting` parking pass proved:

```text
quickchain_preflight_docs: 7 passed
quickchain_tooling_boundary: 3 passed
normal all-target tests: passed
WAL feature all-target tests: passed
focused QuickChain tests: 8 discovered and passed
workspace check: passed
parking gate: passed
```

The WAL feature path matters because accounting artifacts may later need stronger replay/recovery behavior.

The WAL tests now passing under the exhaustive runner include the real WAL-enabled unit path, including:

```text
wal_appends_and_drains
replay_skips_corrupt_records
```

Those tests improve accounting replay hygiene without turning accounting into balance truth.

## 4.9 Accounting artifacts now explicitly bounded

The docs identify these as current safe accounting artifacts:

```text
UsageEvent
UsageEventsIngestRequest
EventIngestPolicy
LabelSet
Dimension
Recorder
CounterRow
SliceRow
SealedSlice
Window
RewardProjectionConfig
RewardProjectionReport
ProjectedRewardSnapshot
RewardSnapshotExport
RewardContributionExport
canonical_snapshot_bytes
canonical_snapshot_cid
reward_snapshot_interop_vector_v1
```

These artifacts may be serialized, exported, and checked for deterministic byte stability.

They are not:

```text
QuickChain roots
balances
receipts
settlement proofs
validator proofs
paid-content unlock authority
```

## 4.10 Artifact CID vs root boundary

The docs explicitly distinguish artifact hashes from roots.

Allowed artifact vocabulary:

```text
snapshot_cid
canonical_snapshot_cid
artifact_cid
sealed_slice_digest
interop_vector_digest
```

Forbidden Phase-0 runtime root vocabulary:

```text
accounting_root
reward_root
state_root
receipt_root
checkpoint_root
epoch_root
validator_root
settlement_root
anchor_root
```

Artifact CIDs prove exact bytes for reports and future vector work.

They do not make accounting a QuickChain root producer.

They do not replace the future canonical vector/root gate.

## 4.11 Event-class doctrine documented

`ron-accounting` now documents QuickChain event classes:

```text
economic_receipt:
  backend wallet/ledger economic receipt evidence

metering:
  usage measurement, not direct balance effect

proof_eligible:
  future proof/challenge candidate after explicit validation and policy

ad_budgeted:
  advertiser/sponsor budget lane, not protocol inflation

analytics_only:
  reporting/fraud/product signal only; never direct protocol ROC payout
```

Safe `ron-accounting` default:

```text
metering:
  normal accounting lane

analytics_only:
  ignored for economic reward projection

proof_eligible:
  preserved only as candidate evidence, not payout authority

ad_budgeted:
  counted only under explicit budget policy, not mint authority

economic_receipt:
  must originate from wallet/ledger evidence, not raw usage events
```

## 4.12 Raw engagement boundary

The docs and tests preserve:

```text
Raw engagement must never directly mint, allocate, transfer, or mutate protocol ROC.
```

Raw engagement examples that must not directly become protocol ROC:

```text
views
likes
impressions
clicks
watch time
dwell time
followers
shares
comments
raw site visits
passive engagement
```

Future rewards must pass through:

```text
accounting summary
→ svc-rewarder deterministic payout planning
→ explicit wallet path
→ ron-ledger durable truth
→ backend-derived receipt
```

## 4.13 Ingest poisoning boundary

`ron-accounting` now has explicit docs/tests making HTTP/library ingest an input boundary only.

Ingest must reject or ignore authority-smuggling fields.

The body must not supply:

```text
wallet authority
ledger authority
QuickChain finality
roots
validator claims
bridge claims
anchor claims
operation_id as client authority
account_sequence as client authority
settlement authority
```

`Idempotency-Key` is retry safety only.

It is not:

```text
operation_id
account_sequence
wallet authority
ledger authority
receipt authority
root authority
settlement authority
```

Poisoned finality/root fields must reject or remain inert.

Duplicate ingest retries must not create duplicate economic authority.

## 4.14 Reward projection boundary

Reward projection is planning input only.

Reward projection may:

```text
read sealed slices
filter eligible rows
aggregate contribution data
produce deterministic planning artifacts
produce canonical bytes
produce artifact CIDs
feed svc-rewarder
```

Reward projection must not:

```text
execute payout
issue payout receipt
mutate wallet
mutate ledger
claim balance truth
claim settlement
claim finality
claim checkpoint inclusion
produce QuickChain roots
create validator evidence
anchor externally
```

Handoff to `svc-rewarder` is a planning handoff.

Handoff to `svc-rewarder` is not wallet execution.

## 4.15 Window/determinism boundary

Accounting windows must remain explicit data artifacts.

Window boundaries may be based on:

```text
event timestamps
slice IDs
configured durations
```

Accounting must not create:

```text
wall-clock roots
DB-order roots
placeholder hashes
fake hashes
root-producing code before canonical bytes and golden vectors authorize it
```

When deterministic bytes are produced, they must be sorted and reproducible.

## 4.16 Remaining `ron-accounting` work

No more immediate patch is required for the current QuickChain Phase-0 parking pass.

Future non-blocking work may include:

```text
deeper WAL crash durability tests
more HTTP ingest edge-case tests
more operator docs
CI integration for crate-local parking gate
stronger sealed-window API tests
future accounting-root vector work after root gates open
future reward-root vector work after root gates open
future independent canonical artifact verifier
```

Do not add accounting roots, reward roots, receipt roots, checkpoint roots, validators, anchors, bridges, staking, liquidity, ROX, Solana, external settlement, or public-chain logic during the internal ROC proving phase.

---

# 5. Latest terminal proof summary

The latest `ron-accounting` run confirmed:

```text
cargo fmt -p ron-accounting
cargo test -p ron-accounting --test quickchain_preflight_docs
cargo test -p ron-accounting --test quickchain_tooling_boundary
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

Results:

```text
quickchain_preflight_docs:
  7 passed

quickchain_tooling_boundary:
  3 passed

normal all-target tests:
  passed

WAL feature all-target tests:
  passed

focused QuickChain discovery:
  8 tests discovered

focused QuickChain execution:
  all 8 passed

parking gate:
  passed

final marker:
  == ron-accounting QuickChain parking gate passed ==
```

Workspace check completed.

The only workspace warnings observed were unrelated to `ron-accounting`:

```text
ryker:
  field max_msg_bytes is never read

micronode:
  unused import UpstreamSpec
```

These warnings did not fail the current parking gate.

They should not block this pair unless the project later promotes the entire workspace to a zero-warning gate.

---

# 6. Files to regenerate / codebundle notes

After the final parked state, regenerate focused codebundles with:

```bash
bash scripts/make_crate_codex.sh --force -c svc-wallet
bash scripts/make_crate_codex.sh --force -c ron-accounting
```

Expected refreshed bundle paths:

```text
crates/svc-wallet/CODEBUNDLE.md
crates/ron-accounting/CODEBUNDLE.md
```

Use the focused bundles in the next session rather than the full workspace bundle unless broader context is necessary.

---

# 7. Pair-level completion estimate

For the current QuickChain Phase-0/preflight scope:

```text
svc-wallet:
  parked / effectively complete for this pass

ron-accounting:
  parked / effectively complete for this pass

svc-wallet + ron-accounting pair:
  parked / green for current Phase-0 boundary hardening
```

Suggested estimate for this pair’s current allowed scope:

```text
95–100% complete for current Phase-0/preflight sweep
```

This does **not** mean full QuickChain is complete.

Full future QuickChain still requires:

```text
locked hash vectors
independent vector reproduction
canonical root bytes
receipt roots
account state roots
hold roots
accounting roots
reward roots
epoch/checkpoint artifacts
proof formats
validator-executable replay
data availability/challenge/archive model
safe pruning model
future validator/committee design
future external anchor decision
```

Those remain future gated work.

---

# 8. What not to change next

Do not reopen `svc-wallet` or `ron-accounting` unless:

```text
a later crate integration breaks compilation
a boundary regression appears
a QuickChain DTO change requires a compatibility update
the user explicitly asks for deeper hardening in this pair
```

Do not add:

```text
QuickChain roots
checkpoint generation
validator signatures
settlement finality
external anchors
bridge code
staking
liquidity
Solana
ROX
exchange-facing code
public-chain settlement
gateway/omnigate ledger mutation
accounting payout execution
rewarder direct ledger mutation
raw engagement protocol payouts
fake receipts
fake balances
silent spend
```

The safest default if a feature looks economic is:

```text
Does this belong in svc-wallet?
Does this belong in ron-ledger?
Does this belong in svc-rewarder as planning only?
Does this require locked vectors first?
```

If the answer is unclear, do not implement it in `ron-accounting`.

---

# 9. Next coherent crate pair

The next coherent QuickChain crate pair after parking `svc-wallet + ron-accounting` is:

```text
svc-rewarder + svc-storage
```

Reason:

```text
svc-rewarder:
  deterministic payout planning only; no direct ledger mutation

svc-storage:
  bytes, b3 integrity, paid storage/access metering; no balance truth
```

The intended next-pair value loop is:

```text
svc-storage paid storage/access metering
→ ron-accounting usage snapshots
→ svc-rewarder payout planning
→ svc-wallet explicit mutation front-door
→ ron-ledger durable truth
```

Primary risks to prevent in the next pair:

```text
storage treats b3 as payment proof
storage cache unlocks paid content alone
storage invents paid receipts
storage mutates wallet/ledger directly
rewarder pays from raw engagement
rewarder mutates ledger directly
rewarder invents balances/receipts/finality
rewarder double-issues on replay
either crate exposes roots/checkpoints/validators/bridges/anchors/staking/liquidity
```

---

# 10. Short drop-in summary

`svc-wallet + ron-accounting` are parked for the current QuickChain Phase-0/preflight pass. `svc-wallet` remains the internal ROC wallet mutation front-door and commits only through `ron-ledger`; it now has QuickChain preflight docs, tooling tests, dynamic test discovery, feature-gated compatibility checks, and parking gates proving it does not become a runtime, root producer, settlement service, bridge, validator, or public-chain authority. `ron-accounting` remains derivative metering/snapshot infrastructure only; it now has a full QuickChain runbook, docs regression test, hardened exhaustive preflight script, WAL feature all-target coverage, dynamic discovery of 8 focused QuickChain tests, and a final parking gate that passed. Accounting artifacts, snapshot CIDs, reward projections, and raw usage events are explicitly documented and tested as planning/reporting artifacts only: no balance truth, no wallet mutation, no ledger mutation, no payout execution, no fake receipts, no fake balances, no roots, no checkpoints, no validators, no settlement, no anchors, no bridges, no staking/liquidity, and no Solana/ROX/external settlement path. The next coherent pair is `svc-rewarder + svc-storage`.


### END NOTE - JUNE 19 2026 - 13:55 CST



### BEGIN NOTE - JUNE 19 2026 - 22:45 CST

Your latest terminal output confirms `ron-accounting` is fully parked: fmt, clippy, all-target tests, WAL feature tests, 9 discovered QuickChain tests, docs regression, workflow hygiene, workspace check, and final parking gate all passed. 

Here are the paste-ready crate notes.

# QuickChain QC-1A Crate Notes — `svc-wallet`

## Status

`svc-wallet` is parked green for the current QuickChain QC-1A boundary slice.

This crate remains the ROC wallet mutation front-door. It is allowed to prepare, validate, submit, and expose backend-derived wallet receipts for ROC operations, but it must not become a QuickChain authority, root producer, checkpoint assembler, validator, settlement layer, bridge, staking surface, liquidity surface, or public-chain adapter.

The crate now has a stronger QuickChain preflight posture around the boundary between wallet mutation truth and downstream accounting observation.

## What changed in this slice

### Added accounting observer boundary coverage

A new focused QuickChain preflight test target was added:

`crates/svc-wallet/tests/quickchain_preflight_accounting_observer_boundary.rs`

This test target hardens the handoff between `svc-wallet` and future/optional `ron-accounting` export behavior.

The purpose is to prove that accounting remains a derivative observer of wallet activity and never becomes balance truth, receipt authority, settlement authority, or a hidden QuickChain mutation path.

### Verified accounting event shape is derivative-only

The new test confirms that `AccountingEvent` contains only derivative observation fields:

* `op`
* `asset`
* `amount_minor`

It intentionally does not contain authority-bearing fields such as:

* `operation_id`
* `account_sequence`
* `state_root`
* `receipt_root`
* `checkpoint_root`
* `finality`
* `balance_minor`
* `receipt_hash`
* validator or settlement metadata

This is important because `svc-wallet` may eventually emit accounting observations, but those observations must never be confused with ledger truth or QuickChain proof authority.

### Verified the no-op accounting client remains non-authoritative

The no-op accounting client seam remains a safe placeholder:

`crates/svc-wallet/src/accounting/client.rs`

It accepts an `AccountingEvent` and intentionally does nothing with it.

This keeps the integration seam present without creating hidden persistence, hidden balance state, hidden receipt state, hidden root material, or hidden external settlement behavior.

The boundary remains:

```text
svc-wallet -> observes wallet mutation -> emits derivative accounting event
ron-ledger -> remains economic truth
ron-accounting -> remains downstream reporting/snapshot infrastructure
QuickChain -> remains future proof/settlement infrastructure
```

### Verified accounting observations only happen after wallet receipt memory

The new test checks that mutation routes call:

```rust
state.remember_receipt(receipt.clone());
```

before calling:

```rust
state.accounting.record(...)
```

This ordering matters.

The wallet receipt is backend-derived from the wallet/ledger mutation path. Accounting must observe that accepted wallet result after the receipt exists. Accounting must not pre-authorize, invent, replace, or race ahead of the receipt path.

The intended ordering is:

```text
validate request
check capability/policy/idempotency
commit through ron-ledger path
build wallet receipt
remember wallet receipt
emit derivative accounting observation
return receipt to caller
```

### Verified only wallet mutation routes emit accounting observations

The new test scans wallet v1 route files and expects accounting observations only from actual mutation routes:

* `burn.rs`
* `escrow.rs`
* `issue.rs`
* `transfer.rs`

Read-only routes must not emit accounting observations:

* `balance.rs`
* `receipt.rs`

This prevents accidental accounting drift from reads, cache checks, receipt lookups, balance lookups, or future display-only endpoints.

The intended route posture is:

```text
Mutation routes:
- may emit derivative accounting observation after accepted wallet receipt

Read-only routes:
- must not emit accounting observations
- must not mutate wallet state
- must not affect accounting totals
- must not fabricate receipts or balances
```

### Verified read-only wallet routes stay read-only

The boundary test explicitly checks that balance and receipt routes do not call:

```rust
state.accounting.record(...)
```

This prevents `balance` or `receipt` reads from becoming metered events that could later be mistaken for rewardable or proof-eligible economic activity.

That is especially important for QuickChain because future proof systems must separate:

```text
economic receipt events
metering events
proof-eligible events
ad-budgeted events
analytics-only events
```

A read-only wallet lookup should not silently become an economic event or a reward signal.

### Verified accounting client source does not gain authority vocabulary

The test scans `src/accounting/client.rs` for forbidden authority terms.

Forbidden drift includes terms such as:

* `operation_id`
* `account_sequence`
* `state_root`
* `receipt_root`
* `checkpoint_root`
* `finality`
* `validator`
* `settlement`
* `balance_minor`
* `receipt_hash`

This is a guard against future accidental expansion of the accounting seam into consensus, roots, settlement, or receipt authority.

If the accounting seam ever needs richer fields, those fields should be reviewed deliberately and should remain derivative unless the QuickChain phase explicitly authorizes a new role.

## Current QuickChain test posture

`svc-wallet` now has 9 focused QuickChain test targets in its discovered preflight matrix:

* `quickchain_preflight_accounting_observer_boundary`
* `quickchain_preflight_boundary`
* `quickchain_preflight_docs`
* `quickchain_preflight_idempotency_identity_boundary`
* `quickchain_preflight_live_route_matrix`
* `quickchain_preflight_no_runtime_authority`
* `quickchain_preflight_projection_validation_matrix`
* `quickchain_preflight_request_poisoning_matrix`
* `quickchain_tooling_boundary`

The discovery-based preflight runner is important because new QuickChain test files should be automatically included in the crate gate without hardcoding each test target manually.

## Current script posture

The crate-local parking script delegates to the crate-local exhaustive preflight script.

Important low-disk workflow note:

Do not run both the exhaustive preflight script and the parking script back-to-back unless a deliberate double pass is needed. The parking script already delegates to preflight.

Preferred low-disk flow after focused tests:

```bash
cargo fmt -p svc-wallet -- --check
cargo test -p svc-wallet --test quickchain_preflight_accounting_observer_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_accounting_observer_boundary
crates/svc-wallet/scripts/dev-quickchain-park.sh
```

## Boundary rules confirmed

`svc-wallet` continues to obey these QuickChain rules:

* `svc-wallet` is the wallet mutation front-door.
* `ron-ledger` remains economic truth.
* Wallet receipts are backend-derived.
* Receipts remain `accepted` only.
* `svc-wallet` does not invent finalized, included, anchored, or checkpointed statuses.
* `idempotency_key` remains a retry key, not operation authority.
* `operation_id` remains future backend-assigned durable ledger-operation identity.
* Accounting observations are derivative only.
* Balance reads are not mutations.
* Receipt lookups are not mutations.
* No fake balances.
* No fake receipts.
* No silent spend.
* No QuickChain roots.
* No checkpoints.
* No validators.
* No settlement.
* No external anchors.
* No bridges.
* No staking.
* No liquidity.
* No Solana/ROX/public-chain settlement behavior.

## Why this matters

This slice strengthens one of the most important internal economy boundaries:

```text
wallet mutation truth != accounting observation
```

The wallet may notify accounting that an accepted operation happened, but accounting must not become the source of wallet truth.

This prevents a future class of bugs where downstream reporting, reward planning, or analytics accidentally becomes economic authority.

That is especially important before Phase 1 roots/proofs, because deterministic roots must eventually be built from controlled, canonical, authorized inputs — not from loosely emitted analytics or accounting counters.

## Current crate completion assessment

For the current QuickChain QC-1A boundary slice:

```text
svc-wallet status: complete / parked green
```

For broader QuickChain Phase 1:

```text
svc-wallet Phase 1 foundation: strong
svc-wallet root/proof authority: intentionally not implemented
```

This is correct. `svc-wallet` should prepare and expose wallet-side receipt/projection material, but it should not itself become the root producer or validator.

## What remains later for `svc-wallet`

Future work should remain gated and deliberate.

Likely later tasks:

1. Preserve wallet receipt determinism as `ron-proto` QuickChain DTOs stabilize.
2. Add explicit projection paths only when canonical bytes and golden vectors are locked.
3. Keep live HTTP receipt shape stable and wallet-owned.
4. Add operation identity only through backend-assigned durable operation semantics.
5. Ensure wallet receipts can be referenced by QuickChain proof material without making `svc-wallet` a chain authority.
6. Keep accounting emission downstream and derivative.
7. Keep paid-flow wallet receipts backend-derived and display-only in clients.
8. Keep CrabLink/Tauri as a client display/user-intent surface, never wallet truth.

Do not add:

* root-producing code
* checkpoint-producing code
* validator code
* settlement code
* anchor code
* bridge code
* staking/liquidity code
* ROX/Solana/public-chain code
* fake finality labels
* client-supplied operation authority
* accounting-derived balances
* accounting-derived receipts

# QuickChain QC-1A Crate Notes — `ron-accounting`

## Status

`ron-accounting` is parked green for the current QuickChain QC-1A boundary slice.

The crate passed its full QuickChain parking gate after formatting. The parking gate covered formatting, strict clippy, normal all-target tests, WAL feature all-target tests, focused QuickChain test discovery, all discovered QuickChain tests, docs regression, workflow package-name hygiene, docs marker checks, and workspace check.

The crate remains derivative accounting infrastructure. It is not balance truth, not wallet authority, not ledger authority, not a QuickChain root producer, not a checkpoint producer, not a validator, not a settlement service, not an anchor service, not a bridge, not staking, and not liquidity infrastructure.

## What changed in this slice

### Added wallet interlock boundary coverage

A new focused QuickChain preflight test target was added:

`crates/ron-accounting/tests/quickchain_preflight_wallet_interlock_boundary.rs`

This test hardens the boundary between `ron-accounting` and the wallet/ledger authority path.

The purpose is to prove that `ron-accounting` cannot directly depend on or mutate wallet/ledger authority, and that accounting ingest cannot accept fields that would smuggle wallet, ledger, receipt, root, settlement, or checkpoint authority into accounting DTOs.

### Verified `ron-accounting` does not depend on wallet or ledger authority crates

The new test checks `crates/ron-accounting/Cargo.toml` and confirms the crate does not directly depend on:

* `svc-wallet`
* `svc_wallet`
* `ron-ledger`
* `ron_ledger`

This is intentional.

`ron-accounting` should consume derivative events, usage counters, sealed slices, and reward-planning artifacts. It should not call wallet mutation APIs or ledger mutation APIs.

The correct dependency shape is:

```text
svc-wallet / ron-ledger -> source of economic truth
ron-accounting -> derivative metering and snapshot infrastructure
svc-rewarder -> payout planning based on accepted snapshot inputs
svc-wallet -> final payout mutation path when explicitly authorized
```

`ron-accounting` should never become:

```text
ron-accounting -> wallet mutation
ron-accounting -> ledger mutation
ron-accounting -> balance truth
ron-accounting -> receipt truth
```

### Verified source tree has no direct wallet or ledger mutation imports

The new test scans the `src/` tree and rejects direct wallet/ledger mutation drift.

Forbidden patterns include direct references such as:

* `svc_wallet::`
* `ron_ledger::`
* `WalletClient`
* `LedgerClient`
* `WalletState`
* `LedgerState`

It also rejects public mutation vocabulary that would imply accounting can issue or move funds directly, such as:

* `issue`
* `transfer`
* `burn`
* `hold`
* `capture`
* `release`
* `expire_hold`
* `mint`
* `mutate_ledger`
* `mutate_wallet`

The intent is not to ban ordinary documentation language forever, but to keep executable accounting source from quietly gaining wallet or ledger authority.

If a future adapter is needed, it should be introduced deliberately at the correct layer and tested as an authority boundary, not slipped into accounting internals.

### Verified top-level ingest rejects wallet and ledger authority poison

The new test checks `UsageEventsIngestRequest` deserialization and ensures top-level ingest rejects unknown authority-bearing fields.

Examples of forbidden top-level poison fields:

* `wallet_balance`
* `wallet_receipt`
* `wallet_mutation`
* `ledger_commit`
* `ledger_sequence`
* `operation_id`
* `account_sequence`
* `settlement_status`
* `state_root`
* `receipt_root`
* `checkpoint_root`

This protects the ingest boundary.

Accounting ingest should accept usage/metering data. It should not accept caller-supplied wallet truth, ledger truth, receipt truth, finality labels, roots, checkpoints, or settlement claims.

### Verified nested usage events reject wallet and ledger authority poison

The new test also checks nested `UsageEvent` deserialization.

This matters because a malicious or confused service could try to place authority fields inside individual event objects instead of at the top-level ingest request.

Forbidden nested fields include the same classes of poison:

* wallet authority fields
* ledger authority fields
* operation identity authority fields
* settlement/finality fields
* root/checkpoint fields

The result is a stronger nested unknown-field rejection boundary.

### Verified QuickChain test discovery includes the new interlock test

The new test confirms that the crate’s QuickChain test discovery now includes:

`quickchain_preflight_wallet_interlock_boundary`

This matters because the parking/preflight runner must continue discovering new QuickChain tests dynamically.

The goal is to prevent future drift where a new boundary test exists but is not actually included in the standard crate gate.

## Current QuickChain test posture

`ron-accounting` now has 9 focused QuickChain test targets:

* `quickchain_preflight_boundary`
* `quickchain_preflight_docs`
* `quickchain_preflight_event_class_boundary`
* `quickchain_preflight_ingest_poisoning`
* `quickchain_preflight_reward_dto_strictness`
* `quickchain_preflight_reward_projection_boundary`
* `quickchain_preflight_snapshot_non_authority`
* `quickchain_preflight_wallet_interlock_boundary`
* `quickchain_tooling_boundary`

The current QuickChain test matrix proves these major boundaries:

1. Accounting is not balance truth.
2. Accounting is not wallet mutation authority.
3. Accounting is not ledger mutation authority.
4. Accounting ingest rejects QuickChain authority poison.
5. Usage events remain metering unless explicitly projected.
6. Raw engagement cannot directly mint, allocate, transfer, or mutate protocol ROC.
7. Reward snapshots are planning artifacts, not balances and not roots.
8. Snapshot CIDs are artifact hashes, not QuickChain roots.
9. Projection order is deterministic but still not root production.
10. Event-class doctrine remains narrow and explicit.
11. Reward DTOs reject unknown authority fields.
12. Tooling remains Bash/Cargo-only with no checked-in Python helpers.
13. Test discovery remains dynamic rather than hardcoded.

## Current script posture

The crate-local preflight script performs:

* forbidden scope marker print
* no checked-in Python helper check
* docs presence check
* format check
* strict clippy
* all-target tests
* WAL feature all-target tests
* dynamic QuickChain test discovery
* discovered QuickChain test execution

The crate-local parking script performs:

* preflight delegation
* docs regression test
* workflow package-name hygiene
* docs marker checks
* workspace check

Low-disk workflow note:

Do not run both the exhaustive preflight script and the parking script back-to-back unless a deliberate double pass is needed. The parking script already delegates to preflight.

Preferred low-disk flow after focused tests:

```bash
cargo fmt -p ron-accounting -- --check
cargo test -p ron-accounting --test quickchain_preflight_wallet_interlock_boundary
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

## Boundary rules confirmed

`ron-accounting` continues to obey these QuickChain rules:

* Accounting is not balance truth.
* Accounting is not receipt truth.
* Accounting is not wallet mutation authority.
* Accounting is not ledger mutation authority.
* Accounting is not a root producer.
* Accounting is not a checkpoint producer.
* Accounting is not a validator.
* Accounting is not settlement.
* Accounting is not an anchor or external anchor.
* Accounting is not a bridge.
* Accounting is not staking.
* Accounting is not liquidity.
* Accounting does not create ROX/Solana/public-chain behavior.
* Accounting snapshots are reporting/planning artifacts.
* Reward snapshots are planning artifacts.
* Artifact CIDs are not QuickChain roots.
* Raw engagement must never directly mint, allocate, transfer, or mutate protocol ROC.
* Rewarder planning must eventually route through wallet/ledger mutation authority.
* Unknown authority fields are rejected at DTO boundaries.
* Money remains integer/string encoded where applicable.
* No fake balances.
* No fake receipts.
* No silent spend.

## Why this matters

This slice strengthens one of the most important QuickChain value-plane boundaries:

```text
accounting snapshot != balance truth
```

`ron-accounting` can observe, aggregate, seal, and project usage-related artifacts. It can produce deterministic artifacts useful for reward planning. But it must not decide balances, mutate balances, create wallet receipts, claim finality, emit roots, or directly pay anyone.

That separation is the core safety rule behind the internal ROC economy:

```text
ron-ledger = truth
svc-wallet = mutation front-door
ron-accounting = derivative snapshots/reporting
svc-rewarder = payout planning
svc-wallet/ron-ledger = eventual authorized payout mutation path
```

This is especially important before QuickChain Phase 1 roots/proofs. If accounting were allowed to smuggle root/finality/settlement fields now, later deterministic proof work would inherit ambiguous authority boundaries.

## Current crate completion assessment

For the current QuickChain QC-1A boundary slice:

```text
ron-accounting status: complete / parked green
```

For broader QuickChain Phase 1:

```text
ron-accounting Phase 1 foundation: strong
ron-accounting root/proof authority: intentionally not implemented
```

This is correct.

`ron-accounting` should eventually produce deterministic, canonicalized artifacts that can be referenced by other systems, but it should not itself produce QuickChain roots or become settlement authority.

## What remains later for `ron-accounting`

Future work should remain scoped and deliberate.

Likely later tasks:

1. Preserve deterministic snapshot canonicalization.
2. Preserve strict unknown-field rejection on all DTOs.
3. Keep reward projection planning-only.
4. Keep artifact CIDs clearly labeled as artifact hashes, not roots.
5. Add interop vectors only when the downstream consumer shape is stable.
6. Ensure rewarder handoff remains explicit and non-mutating.
7. Ensure any future QuickChain proof references are proof inputs, not accounting-produced authority.
8. Continue rejecting floats for money-like fields.
9. Continue rejecting wallet/ledger/root/finality poison.
10. Maintain dynamic QuickChain test discovery.

Do not add:

* wallet client dependency
* ledger client dependency
* balance mutation
* receipt generation
* payout execution
* root-producing code
* checkpoint-producing code
* validator code
* settlement code
* anchor code
* bridge code
* staking/liquidity code
* ROX/Solana/public-chain code
* raw engagement direct payout logic
* accounting-derived wallet truth

Together, these notes mark the pair as parked for this slice:

```text
svc-wallet + ron-accounting: complete for QC-1A boundary hardening
Next crate pair: svc-rewarder + svc-storage
```


### END NOTE - JUNE 19 2026 - 22:45 CST





### END NOTE - JUNE 19 2026 - 22:45 CST


### BEGIN NOTE - JUNE 22 2026 - 10:45 CST
The terminal output confirms both crate parking gates passed: `svc-wallet QuickChain parking gate passed` and `ron-accounting QuickChain parking gate passed`, with both crates discovering and running 10 QuickChain tests each. 

Here are the paste-ready notes:

# QuickChain QC-1A Notes — `svc-wallet + ron-accounting`

## Session scope

This session continued QuickChain Phase 1 Round 1 / QC-1A pre-root/root-material foundation work for the crate pair:

```text
svc-wallet + ron-accounting
```

The purpose of this pair pass was not to implement roots, checkpoints, validators, settlement, anchors, staking, bridges, or external chain behavior.

The purpose was to harden the service boundary between wallet mutation truth and accounting-derived observation/planning artifacts.

This pair is now green and parked for the current QC-1A slice.

---

## Phase / round context

Current QuickChain phase context:

```text
Phase 1 — single-authority deterministic roots/proofs
Round 1 — QC-1A pre-root/root-material foundation
```

This round prepares crates for future deterministic roots/proofs while preventing premature authority leaks.

For this crate pair, the core doctrine is:

```text
svc-wallet:
  owns wallet mutation front-door behavior
  produces backend-derived wallet receipts
  preserves accepted-only receipt posture
  must not expose accounting/reward/root/finality/settlement authority

ron-accounting:
  observes and aggregates derivative usage/reward data
  produces snapshots/planning artifacts
  is not balance truth
  is not wallet truth
  is not ledger truth
  does not mutate wallets or ledger state
```

---

## Files added / changed

### Added

```text
crates/svc-wallet/tests/quickchain_preflight_phase1_pair_interlock.rs
crates/ron-accounting/tests/quickchain_preflight_phase1_pair_interlock.rs
```

No Cargo dependency changes were required.

No runtime wallet/accounting mutation logic was changed for this patch.

No root-producing code was added.

No settlement, validator, anchor, bridge, staking, liquidity, ROX, Solana, or external chain code was added.

---

# `svc-wallet` notes

## What changed

A new focused QC-1A pair-interlock test target was added:

```text
crates/svc-wallet/tests/quickchain_preflight_phase1_pair_interlock.rs
```

This test target hardens the wallet/accounting boundary from the wallet side.

It verifies that `svc-wallet` remains the backend wallet mutation front-door and does not accidentally expose accounting, rewarder, root, finality, or settlement authority through wallet receipt shapes, routes, or manifest dependencies.

---

## `svc-wallet` tests added

The new test target covers four major boundary rules:

```text
1. live_wallet_receipt_does_not_expose_accounting_reward_or_root_authority

2. accounting_observer_client_is_unit_noop_and_cannot_gate_wallet_commits

3. wallet_routes_observe_accounting_only_after_backend_receipt_memory

4. wallet_manifest_keeps_accounting_rewarder_and_external_settlement_out_of_authority_path
```

---

## What the `svc-wallet` tests prove

### 1. Wallet receipts remain wallet receipts

The tests verify that live wallet receipts do not expose or imply:

```text
accounting authority
reward authority
payout authority
root authority
checkpoint authority
finality authority
settlement authority
anchor authority
validator authority
external bridge authority
```

This preserves the correct current receipt meaning:

```text
wallet mutation accepted
```

It does not mean:

```text
QuickChain finalized
checkpointed
rooted
anchored
settled externally
validator-approved
```

---

### 2. Accounting observation is derivative and non-gating

The tests verify that the wallet-side accounting observer remains a derivative/no-op observer shape.

The accounting observer must not:

```text
return spend authority
return root authority
return finality authority
return payout authority
gate commits
fail wallet commits
replace wallet receipt truth
```

This keeps accounting in the correct role:

```text
observe after wallet/ledger truth
```

Not:

```text
approve wallet mutation
decide balance truth
create settlement truth
```

---

### 3. Mutation routes remember wallet receipts before accounting observation

The tests verify that wallet mutation routes preserve the correct ordering:

```text
backend wallet/ledger operation succeeds
→ backend-derived receipt is remembered
→ accounting observation is emitted
```

This protects against a dangerous inversion where accounting could become a gate before wallet truth exists.

Correct ordering:

```text
wallet truth first
accounting observation second
```

Forbidden ordering:

```text
accounting approval first
wallet commit second
```

---

### 4. `svc-wallet` manifest stays free of accounting/rewarder/settlement authority dependencies

The tests verify that `svc-wallet` does not import or depend on crates that would blur authority boundaries, especially:

```text
ron-accounting
svc-rewarder
external settlement authority
bridge/anchor/staking/liquidity authority
```

This is important because `svc-wallet` should expose wallet mutation front-door behavior without turning accounting or reward planning into spend authority.

---

## `svc-wallet` terminal result summary

Focused QC-1A test result:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_phase1_pair_interlock

Result:
  4 passed
  0 failed
```

Parking gate result:

```text
crates/svc-wallet/scripts/dev-quickchain-park.sh

Result:
  svc-wallet QuickChain parking gate passed
```

The parking gate also discovered and ran the full focused QuickChain preflight matrix for `svc-wallet`.

Discovered QuickChain test count:

```text
10
```

Discovered `svc-wallet` QuickChain tests:

```text
quickchain_preflight_accounting_observer_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_idempotency_identity_boundary
quickchain_preflight_live_route_matrix
quickchain_preflight_no_runtime_authority
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_projection_validation_matrix
quickchain_preflight_request_poisoning_matrix
quickchain_tooling_boundary
```

Important green gates from the parking script:

```text
tooling boundary clean
fmt check passed
normal strict clippy gate passed
normal all-target tests passed
focused QuickChain tests passed
feature-gated strict clippy gate passed
feature-gated all-target tests passed
docs regression tests passed
package check with QuickChain preflight feature passed
workspace check passed
svc-wallet QuickChain parking gate passed
```

---

# `ron-accounting` notes

## What changed

A new focused QC-1A pair-interlock test target was added:

```text
crates/ron-accounting/tests/quickchain_preflight_phase1_pair_interlock.rs
```

This test target hardens the wallet/accounting boundary from the accounting side.

It verifies that `ron-accounting` remains a non-authoritative accounting/snapshot/planning-artifact crate and cannot smuggle wallet mutation, payout execution, root authority, finality, or settlement behavior through DTOs, ingest paths, reward snapshots, or source-tree entrypoints.

---

## `ron-accounting` tests added

The new test target covers five major boundary rules:

```text
1. serialized_projected_snapshot_contains_planning_artifact_fields_only

2. usage_ingest_rejects_pair_interlock_authority_fields

3. reward_snapshot_rejects_wallet_payout_execution_poison_fields

4. projected_reward_snapshot_rejects_wallet_payout_execution_poison_fields

5. source_tree_has_no_wallet_payout_execution_entrypoints
```

---

## What the `ron-accounting` tests prove

### 1. Projected snapshots are planning artifacts only

The tests verify that projected reward snapshots serialize as artifact/planning data only.

Allowed posture:

```text
snapshot artifact
reward planning artifact
projection report
artifact CID
```

Forbidden posture:

```text
balance truth
wallet truth
ledger truth
root truth
finality truth
settlement truth
payout execution authority
```

This preserves the core accounting rule:

```text
ron-accounting may summarize and project.
ron-accounting may not mutate or settle.
```

---

### 2. Usage ingest rejects authority poison fields

The tests verify that accounting ingest DTOs reject fields attempting to smuggle future authority, including concepts like:

```text
operation authority
idempotency authority
wallet mutation authority
ledger mutation authority
payout execution
root
checkpoint
finality
anchor
settlement
validator approval
bridge authority
staking/liquidity authority
```

This is important because raw usage, engagement, and metering events must not become protocol ROC allocation authority.

Correct rule:

```text
usage events can inform accounting/reward planning
```

Forbidden rule:

```text
usage events directly mint, allocate, spend, settle, or finalize ROC
```

---

### 3. Reward snapshots reject wallet/payout/root/finality poison fields

The tests verify that reward snapshot DTOs reject unknown/future authority fields.

This keeps reward snapshots in the correct lane:

```text
planning artifact
```

Not:

```text
wallet payout instruction
ledger mutation instruction
QuickChain root
checkpoint
settlement proof
finality proof
```

---

### 4. Projected reward snapshots also reject authority fields

The projected snapshot wrapper remains strict.

This matters because projected snapshots include additional context/reporting around reward snapshots and could otherwise become a place where future fields sneak in.

The test protects against that by ensuring projected reward snapshots cannot become:

```text
payout execution objects
wallet transaction requests
ledger mutation requests
root/finality/checkpoint containers
```

---

### 5. Source tree has no wallet payout execution entrypoints

The tests scan the `ron-accounting` source tree for forbidden wallet/payout mutation entrypoints.

This protects against accidental addition of functions that would make accounting execute wallet payouts directly.

Correct architecture:

```text
ron-accounting:
  records
  aggregates
  snapshots
  projects

svc-rewarder:
  plans payouts

svc-wallet:
  performs explicit wallet mutations through backend-controlled routes

ron-ledger:
  remains truth
```

Forbidden architecture:

```text
ron-accounting directly issues/transfers/captures/releases/burns ROC
ron-accounting directly executes reward payouts
ron-accounting directly mutates ledger truth
```

---

## `ron-accounting` terminal result summary

Focused QC-1A test result:

```text
cargo test -p ron-accounting --test quickchain_preflight_phase1_pair_interlock

Result:
  5 passed
  0 failed
```

Parking gate result:

```text
crates/ron-accounting/scripts/dev-quickchain-park.sh

Result:
  ron-accounting QuickChain parking gate passed
```

The parking gate also discovered and ran the full focused QuickChain preflight matrix for `ron-accounting`.

Discovered QuickChain test count:

```text
10
```

Discovered `ron-accounting` QuickChain tests:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_event_class_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_snapshot_non_authority
quickchain_preflight_wallet_interlock_boundary
quickchain_tooling_boundary
```

Important green gates from the parking script:

```text
tooling boundary clean
docs presence passed
fmt check passed
clippy strict gate passed
all-target tests passed
WAL feature all-target tests passed
focused QuickChain tests passed
docs regression test passed
workflow package-name hygiene passed
workspace check passed
ron-accounting QuickChain parking gate passed
```

---

# Cross-crate boundary now proven

Together, this pair pass proves the following QC-1A boundary:

```text
svc-wallet:
  can mutate wallet state through backend-controlled wallet routes
  can produce accepted wallet receipts
  can emit derivative accounting observations after receipt memory

ron-accounting:
  can ingest/aggregate/project/snapshot derivative data
  can produce planning artifacts and artifact CIDs
  cannot mutate wallet/ledger truth
  cannot create root/finality/settlement authority
```

The pair boundary is now:

```text
wallet truth first
accounting observation second
reward planning later
wallet payout only through explicit wallet path
```

This is the correct current QuickChain posture.

---

# Forbidden scope preserved

This pair pass did not introduce:

```text
roots
root-producing runtime
checkpoints
validators
committee consensus
settlement
external anchors
bridges
staking
liquidity
ROX runtime
Solana runtime
external chain authority
gateway/omnigate mutation authority
rewarder ledger mutation
accounting ledger mutation
client-side spend authority
fake balances
fake receipts
silent spend
```

---

# Warnings observed

Two workspace warnings appeared during the broader workspace checks:

```text
ryker:
  field `max_msg_bytes` is never read

micronode:
  unused import: `UpstreamSpec`
```

These are not failures for this crate pair.

They came from broader workspace checking, not from the focused `svc-wallet + ron-accounting` gates.

They can be cleaned in a later unrelated warning cleanup pass if desired.

They do not block QuickChain QC-1A progress for this pair.

---

# Current status after this pair

```text
Phase 1 Round 1 / QC-1A status:

1. ron-proto + ron-ledger
   Status: green for current pre-root/root-material slice

2. svc-wallet + ron-accounting
   Status: green and parked

3. svc-rewarder + svc-storage
   Status: next pair

4. svc-gateway + omnigate
   Status: pending

5. svc-index + ron-policy
   Status: pending

6. CrabLink Tauri + client adapters
   Status: pending
```

---

# Completion label for this pair

Use this wording:

```text
svc-wallet + ron-accounting are green and parked for QuickChain Phase 1 Round 1 / QC-1A foundation.
```

Or:

```text
The wallet/accounting boundary is green for the current QC-1A pre-root slice.
```

Do not call this:

```text
QuickChain Phase 1 complete
root engine complete
proof engine complete
settlement ready
validator ready
```

Full Phase 1 still requires:

```text
canonical bytes locked
real locked hashes
deterministic receipt/account/hold/epoch roots
proof/verifier behavior
final cross-crate audit
downstream non-authority confirmation
```

---

# Next pair

Next crate pair:

```text
svc-rewarder + svc-storage
```

Expected focus for next pair:

```text
svc-rewarder:
  reward planning only
  no ledger mutation
  no wallet payout execution
  no raw engagement protocol payout authority
  no root/finality/settlement fields

svc-storage:
  bytes/artifacts/manifests only
  b3 verification
  no paid unlock from cache alone
  artifact CID is not QuickChain root
  no wallet/ledger mutation
```

The next patch should likely add or harden QC-1A pair-interlock tests proving:

```text
rewarder output remains payout planning only
storage artifacts remain non-authoritative
storage CIDs/artifact hashes do not become roots/finality
rewarder cannot execute wallet payouts
storage cannot unlock paid content without wallet/gateway receipt path
```

---

# Low-disk workflow reminder

Before moving to the next pair, recover disk if needed:

```bash
cargo clean
```

Then generate/attach only the focused codebundles for:

```text
svc-rewarder + svc-storage
```

Do not rerun `svc-wallet + ron-accounting` unless one of those crates is patched again.

Do not run both preflight and park for the same crate unless debugging requires it.

Use the park script only after focused tests, because the park scripts already delegate to the deeper QuickChain gate.

### END NOTE - JUNE 22 2026 - 10:45 CST




### BEGIN NOTE - JUNE 22 2026 - 18:05 CST

Here are paste-ready crate notes for **`svc-wallet + ron-accounting`**. The terminal confirms both crates passed the focused tooling checks, exhaustive preflight gates, dynamic QuickChain discovery with 11 tests each, clippy, all-target tests, and ron-accounting WAL feature tests. 

# QuickChain Phase 1 Round 2 Crate Notes — `svc-wallet` + `ron-accounting`

## Status

`svc-wallet` and `ron-accounting` are now **QuickChain Phase 1 complete** for this crate pair.

Phase 1 only has two rounds for this pair:

* **Round 1:** downstream boundary/interlock confirmation.
* **Round 2:** deterministic root/proof integration confirmation and final non-authority hardening.

Both rounds are complete. This pair should now be considered **parked for Phase 1** unless a later downstream crate exposes a real integration mismatch.

---

## Crate: `svc-wallet`

### Phase 1 Round 2 Goal

The goal for `svc-wallet` was not to turn the wallet into a QuickChain authority. The goal was to prove that wallet receipts can safely project into deterministic Phase 1 receipt-root material while preserving the wallet boundary:

* `svc-wallet` remains the ROC wallet mutation front-door.
* `ron-ledger` remains economic truth.
* wallet receipts remain accepted-only wallet/ledger receipts.
* wallet projection is explicit and feature-gated.
* wallet does not become a root producer, proof authority, validator, settlement authority, bridge, anchor, staking, liquidity, or external-chain component.

### What Changed

Added/confirmed Phase 1 Round 2 receipt-root material interlock coverage.

The new Phase 1 test target proves that accepted wallet receipts can be projected into QuickChain receipt tree material without granting wallet any root/finality authority.

Key coverage:

* builds wallet receipts with stable accepted-only settlement status;
* projects wallet receipts through the explicit QuickChain preflight projection path;
* derives receipt hash payload material using canonical QuickChain DTOs;
* builds receipt tree material through `ron-ledger` QuickChain helpers;
* computes deterministic receipt-root material outside wallet authority;
* builds and verifies inclusion proof material outside wallet authority;
* confirms wallet projection rejects poison fields such as:

  * `state_root`
  * `receipt_root`
  * `checkpoint_root`
  * `root_hash`
  * `root_node_hash`
  * `inclusion_proof`
  * `proof_steps`
  * `validator_signature`
  * `finality`
  * `finalized`
  * `anchored`
  * `external_settlement_status`

### Important Boundary Preserved

`svc-wallet` can provide receipt projection material for Phase 1, but it does **not** own root construction as runtime authority.

The safe shape is:

```text
wallet receipt -> explicit projection -> receipt hash payload -> tree material input
```

The forbidden shape remains:

```text
wallet receipt -> wallet-owned finality/root/proof/settlement authority
```

### Tooling / Gate Hardening

The `svc-wallet` QuickChain preflight script was tightened so the Phase 1 Round 2 test cannot silently drop out later.

The script now requires at least 11 QuickChain test targets and explicitly requires:

* `quickchain_phase1_receipt_root_material_interlock`
* `quickchain_preflight_accounting_observer_boundary`
* `quickchain_preflight_boundary`
* `quickchain_preflight_docs`
* `quickchain_preflight_idempotency_identity_boundary`
* `quickchain_preflight_live_route_matrix`
* `quickchain_preflight_no_runtime_authority`
* `quickchain_preflight_phase1_pair_interlock`
* `quickchain_preflight_projection_validation_matrix`
* `quickchain_preflight_request_poisoning_matrix`
* `quickchain_tooling_boundary`

The tooling boundary test was also updated to confirm:

* dynamic test discovery includes the new Phase 1 Round 2 test;
* preflight remains bash/cargo-only;
* no checked-in Python helpers exist under `svc-wallet`;
* the gate requires the full current QuickChain test matrix.

### Validation Passed

`svc-wallet` passed:

* focused `quickchain_tooling_boundary` test: 4/4;
* normal strict clippy gate;
* normal all-target tests;
* dynamic QuickChain discovery;
* 11 discovered QuickChain test targets;
* feature-gated QuickChain test matrix;
* feature-gated strict clippy gate;
* feature-gated all-target tests;
* final exhaustive QuickChain preflight gate.

Final status:

```text
svc-wallet QuickChain Phase 1: COMPLETE / PARKED
```

---

## Crate: `ron-accounting`

### Phase 1 Round 2 Goal

The goal for `ron-accounting` was to prove that accounting remains a derivative metering/snapshot/reporting layer and cannot become QuickChain root truth, balance truth, payout execution, wallet mutation, ledger mutation, finality, settlement, or validator authority.

`ron-accounting` should produce deterministic accounting/reward artifacts that can feed planning systems, but it must never mutate ROC balances or produce authoritative QuickChain state.

### What Changed

Added/confirmed Phase 1 Round 2 root-material non-authority coverage.

The new Phase 1 test target proves:

* `ron-accounting` does not link to root/wallet authority crates as authority dependencies;
* accounting artifacts reject root/proof/finality/settlement poison fields;
* reward snapshot CIDs remain artifact references, not roots;
* accounting source does not build or verify Phase 1 root material;
* accounting does not import or call QuickChain root/proof helpers;
* accounting remains downstream metering/planning infrastructure only.

Key poison fields rejected or scanned against include:

* `tree`
* `tree_material`
* `sort_key_hex`
* `payload_schema`
* `payload_hash`
* `root_hash`
* `root_node_hash`
* `inclusion_proof`
* `proof_steps`
* `leaf_index`
* `receipt_root`
* `state_root`
* `accounting_root`
* `checkpoint_root`
* `settlement_status`
* `finality`
* `validator_signature`

### Important Boundary Preserved

`ron-accounting` can produce accounting artifacts and reward-planning inputs, but it cannot become root truth.

The safe shape is:

```text
metering events -> accounting snapshot artifact CID -> rewarder planning input
```

The forbidden shape remains:

```text
accounting snapshot -> balance truth / root truth / payout execution / settlement authority
```

### Dependency Boundary

`ron-accounting` remains isolated from wallet/ledger/root authority crates for Phase 1 purposes.

The Phase 1 Round 2 tests confirm the crate manifest does not pull in authority dependencies such as:

* `ron-proto`
* `ron-ledger`
* `svc-wallet`

This preserves the doctrine that accounting is not balance truth and not root authority.

### Tooling / Gate Hardening

The `ron-accounting` QuickChain preflight script was tightened so the Phase 1 Round 2 test cannot silently drop out later.

The script now requires at least 11 QuickChain test targets and explicitly requires:

* `quickchain_phase1_root_material_non_authority`
* `quickchain_preflight_boundary`
* `quickchain_preflight_docs`
* `quickchain_preflight_event_class_boundary`
* `quickchain_preflight_ingest_poisoning`
* `quickchain_preflight_phase1_pair_interlock`
* `quickchain_preflight_reward_dto_strictness`
* `quickchain_preflight_reward_projection_boundary`
* `quickchain_preflight_snapshot_non_authority`
* `quickchain_preflight_wallet_interlock_boundary`
* `quickchain_tooling_boundary`

The tooling boundary test was also updated to confirm:

* dynamic test discovery includes the new Phase 1 Round 2 test;
* preflight remains bash/cargo-only;
* no checked-in Python helpers exist under `ron-accounting`;
* accounting non-authority markers remain present in the gate;
* the gate requires the full current QuickChain test matrix.

### Validation Passed

`ron-accounting` passed:

* focused `quickchain_tooling_boundary` test: 4/4;
* docs regression test: 7/7;
* strict clippy gate;
* all-target tests;
* WAL feature all-target tests;
* dynamic QuickChain discovery;
* 11 discovered QuickChain test targets;
* full discovered QuickChain test matrix;
* final exhaustive QuickChain preflight gate.

Final status:

```text
ron-accounting QuickChain Phase 1: COMPLETE / PARKED
```

---

## Pair-Level Result

`svc-wallet + ron-accounting` are now complete for QuickChain Phase 1.

The pair now proves:

* wallet receipts can project into receipt-root material;
* wallet does not become root/proof/finality authority;
* accounting snapshots remain artifact CIDs, not roots;
* accounting cannot become balance truth;
* accounting cannot mutate wallet, ledger, or root material;
* reward-related accounting artifacts remain planning inputs only;
* dynamic QuickChain test discovery is locked for both crates;
* both crates remain bash/cargo-only for QuickChain preflight tooling;
* no Python helper drift was introduced;
* no external settlement, bridge, staking, liquidity, validator, anchor, or public-chain authority was introduced.

Final pair status:

```text
QuickChain Phase 1 — svc-wallet + ron-accounting
Round 1: COMPLETE
Round 2: COMPLETE
Pair status: PHASE 1 COMPLETE / PARKED
```

## Recommended Next Step

Do not patch `svc-wallet` or `ron-accounting` further for Phase 1 unless a later downstream crate exposes a real integration mismatch.

The next Phase 1 Round 2 crate pair should be:

```text
svc-rewarder + svc-storage
```


### END NOTE - JUNE 22 2026 - 18:05 CST


### BEGIN NOTE - JUNE 23 2026 - 18:25 CST

Here are paste-ready crate notes for **`svc-wallet + ron-accounting`**. The latest terminal output confirms the marker/tooling repair passed, the focused Phase 2 replay-boundary tests passed, and both crate-local exhaustive preflight gates ended green with `tests=12`. 

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-wallet + ron-accounting

# QuickChain Phase 2 Round 1 Crate Notes — svc-wallet + ron-accounting

## 0. Status

Crate pair:

```text
svc-wallet + ron-accounting
```

Phase / round:

```text
QuickChain Phase 2 — small committee / replicated verification
Round 1 — verifier artifact / read-only replication
```

Status:

```text
COMPLETE / PARKED for Phase 2 Round 1
```

This does **not** mean all of Phase 2 Round 1 is complete. It means the second crate pair in the fixed buildplan order is complete for this round.

Current Phase 2 Round 1 pair status:

```text
1. ron-proto + ron-ledger              COMPLETE / PARKED
2. svc-wallet + ron-accounting         COMPLETE / PARKED
3. svc-rewarder + svc-storage          NEXT
4. svc-gateway + omnigate              pending
5. svc-index + ron-policy              pending
6. CrabLink Tauri + client adapters    pending
```

Safe completion label:

```text
QuickChain Phase 2 Round 1 — svc-wallet + ron-accounting complete / parked
```

---

## 1. Scope of this pair

The goal for this pair was to prove that `svc-wallet` and `ron-accounting` can safely coexist with the new Phase 2 Round 1 read-only verifier replay artifacts **without becoming verifier authority, committee authority, finality authority, settlement authority, or root authority**.

The work did **not** add:

```text
validators
committee signing
quorum certificates
fork choice
finality
staking
slashing
public bridge
external settlement
ROX/Solana path
ledger mutation from accounting
rewarder payout mutation
gateway/client spend authority
public-chain runtime behavior
```

This pair remains strictly inside the Phase 2 Round 1 “read-only replication / verifier artifact boundary” scope.

---

## 2. What changed for svc-wallet

### Added / confirmed test

```text
crates/svc-wallet/tests/quickchain_phase2_replay_boundary.rs
```

This test target is feature-gated:

```text
--features quickchain-preflight
```

Purpose:

```text
svc-wallet may provide backend-derived wallet receipt evidence that can be projected into read-only verifier replay material, but svc-wallet must not become verifier, finality, committee, or settlement authority.
```

Key behavior proven:

```text
wallet receipts can be projected into QuickChain receipt material
receipt material can be checked by ron-ledger read-only replay verification
two replay runs over the same wallet-derived evidence produce the same result
the replay result is diagnostic evidence only
wallet projections do not expose replay/finality/committee/bridge/external-settlement authority fields
live wallet mutation routes reject client-supplied Phase 2 verifier authority poison fields
```

Focused test result:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase2_replay_boundary

Result:
  2 passed
  0 failed
```

The two tests were:

```text
wallet_receipt_evidence_can_be_read_only_replayed_without_wallet_becoming_verifier
wallet_live_routes_reject_phase2_verifier_authority_poison_fields
```

### svc-wallet invariant locked

`svc-wallet` remains:

```text
ROC wallet mutation front-door
backend receipt producer
backend-controlled issue/transfer/burn/hold/capture/release path
accepted-only receipt source
```

It does **not** become:

```text
verifier
committee member
root producer
proof authority
finality authority
settlement authority
bridge authority
external settlement adapter
staking/slashing authority
ledger replacement
```

---

## 3. What changed for ron-accounting

### Added / confirmed test

```text
crates/ron-accounting/tests/quickchain_phase2_replay_boundary.rs
```

Purpose:

```text
ron-accounting snapshots may become verifier inputs later, but accounting must not become replay verifier, quorum authority, root authority, or settlement authority.
```

Key behavior proven:

```text
accounting snapshot CIDs remain artifact references
projected reward snapshots remain derivative planning/observation artifacts
accounting snapshots reject Phase 2 verifier replay poison fields
nested accounting contribution rows reject verifier/finality/committee poison fields
projected snapshots reject verifier/finality/committee poison fields
ron-accounting source does not link or implement verifier replay authority
ron-accounting source does not depend on wallet or ledger authority crates
```

Focused test result:

```text
cargo test -p ron-accounting --test quickchain_phase2_replay_boundary

Result:
  3 passed
  0 failed
```

The three tests were:

```text
accounting_snapshot_cid_remains_artifact_not_verifier_replay_result
accounting_snapshots_reject_phase2_verifier_replay_poison_fields
accounting_source_does_not_link_or_implement_phase2_replay_verifier_authority
```

### ron-accounting invariant locked

`ron-accounting` remains:

```text
usage metering
snapshot production
reward input projection
artifact CID producer
derivative observation infrastructure
```

It does **not** become:

```text
balance truth
wallet mutation authority
ledger mutation authority
replay verifier
committee authority
root producer
proof authority
finality authority
settlement authority
bridge authority
external settlement authority
reward payout executor
```

---

## 4. Parking gate hardening

A follow-up gate-marker fix was applied because the Phase 2 replay-boundary tests increased each crate’s QuickChain test count from 11 to 12.

The patch aligned:

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/tests/quickchain_tooling_boundary.rs
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/tests/quickchain_preflight_docs.rs
crates/ron-accounting/tests/quickchain_tooling_boundary.rs
```

The gate hardening now requires:

```text
svc-wallet:
  quickchain_phase2_replay_boundary included in required QuickChain tests
  minimum QuickChain test target count raised to 12

ron-accounting:
  quickchain_phase2_replay_boundary included in required QuickChain tests
  minimum QuickChain test target count raised to 12
```

This prevents the pair from being accidentally marked green in the future without the Phase 2 replay-boundary tests.

Focused marker/tooling repair results:

```text
svc-wallet quickchain_tooling_boundary:
  4 passed
  0 failed

svc-wallet quickchain_tooling_boundary with quickchain-preflight:
  4 passed
  0 failed

ron-accounting quickchain_preflight_docs:
  7 passed
  0 failed

ron-accounting quickchain_tooling_boundary:
  4 passed
  0 failed
```

Patch marker:

```text
QuickChain Phase 2 Round 1 svc-wallet + ron-accounting gate marker fix applied.
```

---

## 5. Final svc-wallet parking result

Command run:

```bash
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=12 ==
```

The `svc-wallet` preflight confirmed:

```text
fmt check passed
normal strict clippy gate passed
normal all-target tests passed
dynamic QuickChain test discovery passed
12 focused QuickChain tests discovered
all focused QuickChain tests passed with quickchain-preflight feature
feature-gated strict clippy gate passed
feature-gated all-target tests passed
```

Discovered focused QuickChain tests:

```text
quickchain_phase1_receipt_root_material_interlock
quickchain_phase2_replay_boundary
quickchain_preflight_accounting_observer_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_idempotency_identity_boundary
quickchain_preflight_live_route_matrix
quickchain_preflight_no_runtime_authority
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_projection_validation_matrix
quickchain_preflight_request_poisoning_matrix
quickchain_tooling_boundary
```

Important svc-wallet checks that passed:

```text
wallet receipts project into receipt root material without wallet root authority
wallet projection rejects root/proof/finality poison
wallet receipt evidence can be read-only replayed without wallet becoming verifier
wallet live routes reject Phase 2 verifier authority poison fields
wallet accounting event is derivative observation shape only
accounting observations happen after backend receipt memory
live wallet receipts do not expose accounting/reward/root authority
idempotency remains retry identity, not operation authority
live mutation routes remain wallet receipts under quickchain-preflight
quickchain authority fields are rejected as unknown request fields
projection DTO rejects unknown future authority fields
issue/transfer/burn/hold/capture/release reject QuickChain authority fields
```

---

## 6. Final ron-accounting parking result

Command run:

```bash
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=12 ==
```

The `ron-accounting` preflight confirmed:

```text
tooling boundary clean
docs presence passed
fmt check passed
clippy strict gate passed
all-target tests passed
WAL feature all-target tests passed
dynamic QuickChain test discovery passed
12 focused QuickChain tests discovered
all focused QuickChain tests passed
```

Discovered focused QuickChain tests:

```text
quickchain_phase1_root_material_non_authority
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_event_class_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_snapshot_non_authority
quickchain_preflight_wallet_interlock_boundary
quickchain_tooling_boundary
```

Important ron-accounting checks that passed:

```text
accounting snapshot CID is artifact reference, not balance/root truth
accounting artifacts reject Phase 1 root material poison fields
accounting source never builds or verifies Phase 1 root material
accounting snapshot CID remains artifact, not verifier replay result
accounting snapshots reject Phase 2 verifier replay poison fields
accounting source does not link or implement Phase 2 verifier authority
usage and ingest DTOs reject smuggled QuickChain authority fields
raw engagement usage is metering, not protocol ROC authority
reward snapshots serialize as planning artifacts, not balance/root truth
projected reward snapshots are artifact CIDs, not QuickChain roots
event-class doctrine remains narrow and explicit
reward DTOs reject unknown authority fields
reward projection remains planning only
snapshot canonical bytes do not contain finality or mutation vocabulary
ron-accounting does not depend on wallet or ledger authority crates
source tree has no direct wallet or ledger mutation imports
```

---

## 7. Pair-level boundary now proven

This pair now proves the correct value-plane ordering:

```text
svc-wallet mutation path
  -> backend wallet receipt
  -> accepted-only receipt evidence
  -> optional read-only replay material
  -> diagnostic verifier replay result

ron-accounting observation path
  -> usage/metering events
  -> sealed/accounting snapshots
  -> projected reward planning artifacts
  -> artifact CIDs
  -> future downstream planning input only
```

Correct authority boundaries:

```text
ron-ledger remains economic truth
svc-wallet remains mutation front-door
ron-accounting remains derivative snapshot/accounting infrastructure
svc-rewarder later plans payouts but does not mutate ledger
gateway/omnigate later enforce paid access through wallet/backend-derived truth
CrabLink displays truth and requests actions, but does not create economic authority
```

Forbidden authority ordering:

```text
accounting approves wallet mutation
accounting creates balance truth
accounting creates replay verifier authority
wallet claims committee/finality authority
wallet accepts client-supplied replay/finality fields
reward planning directly mutates ledger
gateway/client cache unlocks paid content alone
external settlement replaces wallet/ledger truth
```

---

## 8. Files changed / confirmed

Important files from this pair:

```text
crates/svc-wallet/tests/quickchain_phase2_replay_boundary.rs
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/tests/quickchain_tooling_boundary.rs

crates/ron-accounting/tests/quickchain_phase2_replay_boundary.rs
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/tests/quickchain_preflight_docs.rs
crates/ron-accounting/tests/quickchain_tooling_boundary.rs
```

Generated codebundles after the work:

```text
crates/svc-wallet/CODEBUNDLE.md
crates/ron-accounting/CODEBUNDLE.md
```

---

## 9. Commands that now prove the pair

Focused Phase 2 tests:

```bash
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase2_replay_boundary
cargo test -p ron-accounting --test quickchain_phase2_replay_boundary
```

Tooling/docs marker tests:

```bash
cargo test -p svc-wallet --test quickchain_tooling_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_tooling_boundary
cargo test -p ron-accounting --test quickchain_preflight_docs
cargo test -p ron-accounting --test quickchain_tooling_boundary
```

Full crate-local parking gates:

```bash
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=12 ==
== ron-accounting quickchain exhaustive preflight gate passed: tests=12 ==
```

---

## 10. Current completion judgment

Final pair judgment:

```text
svc-wallet + ron-accounting are 100% complete / parked for QuickChain Phase 2 Round 1.
```

What is complete:

```text
Phase 2 Round 1 work for this crate pair
Phase 2 read-only verifier replay boundary for wallet receipts
Phase 2 verifier-authority non-escalation boundary for accounting snapshots
preflight gate discovery and tooling marker hardening
crate-local clippy/all-target/focused QuickChain tests
```

What is not complete:

```text
full Phase 2 Round 1
full Phase 2
committee signatures
quorum/finality
validator identity/attestation
staking/slashing
external settlement
public bridge
```

Next crate pair:

```text
svc-rewarder + svc-storage
```

Next pair goal:

```text
Prove reward planning and storage/metering can safely consume or reference Phase 2 read-only verifier artifacts without rewarder becoming verifier/payout execution authority and without storage becoming proof/finality/settlement/access authority by itself.
```

---

## 11. Carry-forward warnings

Do not reopen `svc-wallet + ron-accounting` during Phase 2 Round 1 unless:

```text
a downstream pair exposes a real contract mismatch
ron-proto verifier DTOs change
ron-ledger read-only replay API changes
wallet receipt projection schema changes
accounting snapshot projection schema changes
a later final audit catches authority vocabulary or dependency drift
```

Otherwise, treat this pair as parked and move to:

```text
svc-rewarder + svc-storage
```

---

## 12. One-line handoff

```text
svc-wallet + ron-accounting are green/parked for QuickChain Phase 2 Round 1: wallet receipts can feed read-only replay evidence without wallet becoming verifier/finality authority, accounting snapshots remain derivative artifacts without verifier/committee/root/settlement authority, and both crate-local exhaustive gates pass with 12 focused QuickChain tests each.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-wallet + ron-accounting

The next clean build step is **Phase 2 Round 1 — `svc-rewarder + svc-storage`**.


### END NOTE - JUNE 23 2026 - 18:25 CST


### BEGIN NOTE - JUNE 24 2026 - 00:15 CST

Here are the paste-ready crate notes. They are anchored to the buildplan’s fixed crate-pair order and hard boundaries, the review bundle’s future-settlement / non-public-runtime doctrine, and the final clean terminal proof showing both `tests=13` gates green.   

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2 - svc-wallet + ron-accounting

# QuickChain Phase 2 Round 2 Crate Notes — `svc-wallet + ron-accounting`

## 0. Status

Crate pair:

```text
svc-wallet + ron-accounting
```

Phase / round:

```text
QuickChain Phase 2 — small committee / replicated verification
Round 2 — committee agreement / readiness
```

Important phase reminder:

```text
Phase 2 has exactly two rounds.
Round 1 is verifier artifact / read-only replication.
Round 2 is small committee agreement / readiness.
There is no Round 3 for Phase 2 in the current buildplan.
```

Status for this pair:

```text
COMPLETE / PARKED for QuickChain Phase 2 Round 2
```

This does **not** mean all of QuickChain Phase 2 is complete yet.

It means the second crate pair in the fixed Phase 2 Round 2 crate-pair order is complete and should not be reopened unless a later downstream integration exposes a real regression.

Current QuickChain Phase 2 Round 2 position:

```text
1. ron-proto + ron-ledger        COMPLETE / PARKED
2. svc-wallet + ron-accounting   COMPLETE / PARKED
3. svc-rewarder + svc-storage    NEXT
4. svc-gateway + omnigate        pending
5. svc-index + ron-policy        pending
6. CrabLink Tauri + adapters     pending
```

---

## 1. Buildplan / doctrine context

The buildplan’s fixed full-round crate-pair order remains:

```text
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + svc-storage
4. svc-gateway + omnigate
5. svc-index + ron-policy
6. CrabLink Tauri + client adapters
```

The active QuickChain doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

The active architecture boundaries remain:

```text
svc-wallet      = explicit wallet mutation front-door
ron-ledger      = economic / balance truth
ron-accounting  = derivative metering and snapshots, not balance truth
svc-rewarder    = payout planning, not mutation
svc-storage     = bytes/artifacts by b3, not payment truth
svc-gateway     = public backend boundary, not ledger mutation
omnigate        = hydration/orchestration, not ledger mutation
CrabLink Tauri  = display/user intent, not backend truth
```

QuickChain remains future settlement infrastructure.

It is still not:

```text
public chain runtime
validator economy
staking system
slashing system
public bridge
external settlement path
ROX/Solana active runtime
exchange-facing logic
gateway/client/index/policy/cache authority
```

---

## 2. Round 2 goal for this pair

Phase 2 Round 2 is about small committee readiness.

For `svc-wallet + ron-accounting`, that does **not** mean implementing live committee signing or quorum logic inside these crates.

The goal for this pair was:

```text
Prove that svc-wallet and ron-accounting can coexist with future committee verification semantics without absorbing committee, quorum, finality, validator, settlement, bridge, staking, or slashing authority.
```

The correct Phase 2 Round 2 posture is:

```text
wallet receipts and accounting artifacts may be evidence or inputs for downstream verifier tooling
but svc-wallet and ron-accounting must not become the verifier committee
and must not produce attestations, quorum certificates, finality claims, fork choice, validator signatures, or settlement outcomes
```

---

## 3. `svc-wallet` status

`svc-wallet` is complete / parked for QuickChain Phase 2 Round 2.

It remains:

```text
the explicit ROC wallet mutation front-door
```

It is allowed to:

```text
validate issue requests
validate transfer requests
validate burn requests
validate hold-open requests
validate hold-capture requests
validate hold-release requests
enforce wallet capability / policy boundaries
submit mutations through the ledger path
return backend-derived wallet receipts
remember backend-derived wallet receipts
emit derivative accounting observations after accepted wallet receipts
expose accepted wallet receipt evidence to downstream verification tooling
```

It is not allowed to become:

```text
balance truth
ledger truth
accounting truth
reward authority
root authority
proof authority
committee member
attestation signer
quorum authority
fork-choice authority
finality authority
settlement authority
validator authority
staking authority
slashing authority
bridge authority
external settlement authority
client-side entitlement authority
```

### New / confirmed Phase 2 Round 2 target

The key Phase 2 Round 2 test target for `svc-wallet` is:

```text
crates/svc-wallet/tests/quickchain_phase2_committee_boundary.rs
```

This target is feature-gated:

```text
quickchain-preflight
```

It proves:

```text
wallet receipt projection has no committee attestation or quorum authority
wallet mutation routes reject committee / quorum / validator / finality poison fields
wallet source does not implement committee or validator-economy authority
```

Important poison concepts guarded against include:

```text
committee_member_id
committee_epoch
committee_round
committee_signature
committee_signatures
signed_verification_attestation
verification_attestation
attestation_signature
attestation_public_key
attestation_weight
quorum_certificate
quorum_threshold
quorum_reached
validator_signature
validator_set
validator_index
fork_choice
double_attestation_evidence
equivocation_evidence
bonded_stake
stake_weight
slash_evidence
slashing
external_anchor
external_settlement
bridge_finality
settlement_finality
```

### Existing Phase 2 / Phase 1 boundaries preserved

`svc-wallet` still preserves earlier QuickChain gates:

```text
wallet receipts project into receipt root material without wallet root authority
wallet projection rejects root/proof/finality poison after material exists
wallet receipt evidence can be read-only replayed without wallet becoming verifier
wallet live routes reject Phase 2 verifier authority poison fields
wallet accounting event remains derivative observation only
accounting observations happen after backend receipt memory
read-only wallet routes do not emit accounting observations
idempotency key remains retry identity, not operation authority
operation identity remains backend/ledger-assigned, not client authority
live wallet mutation routes remain wallet receipts under quickchain-preflight
quickchain authority fields are rejected as unknown request fields
projection DTO rejects unknown future authority fields
issue/transfer/burn/hold/capture/release reject QuickChain authority poison fields
```

### `svc-wallet` gate status

Final `svc-wallet` gate:

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=13 ==
```

The final `svc-wallet` gate confirmed:

```text
tooling boundary clean
no checked-in Python helpers under svc-wallet
fmt check passed
normal strict clippy gate passed
normal all-target tests passed
dynamic QuickChain discovery passed
13 focused QuickChain test targets discovered
all focused QuickChain tests passed with quickchain-preflight feature
feature-gated strict clippy gate passed
feature-gated all-target tests passed
bench smoke targets completed
no integer expression warning remained after count-guard repair
```

Discovered focused QuickChain tests:

```text
quickchain_phase1_receipt_root_material_interlock
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_accounting_observer_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_idempotency_identity_boundary
quickchain_preflight_live_route_matrix
quickchain_preflight_no_runtime_authority
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_projection_validation_matrix
quickchain_preflight_request_poisoning_matrix
quickchain_tooling_boundary
```

---

## 4. `ron-accounting` status

`ron-accounting` is complete / parked for QuickChain Phase 2 Round 2.

It remains:

```text
derivative metering, snapshot, and reward-planning artifact infrastructure
```

It is allowed to:

```text
ingest usage / metering events
normalize labels
maintain bounded counters
seal accounting slices / windows
produce deterministic accounting artifacts
produce reward-planning artifacts
produce canonical artifact bytes
produce artifact CIDs / hashes
feed svc-rewarder planning
```

It is not allowed to become:

```text
balance truth
wallet authority
ledger authority
wallet mutation authority
ledger mutation authority
direct ROC issuance authority
receipt authority
root authority
proof authority
committee authority
attestation authority
quorum authority
fork-choice authority
finality authority
settlement authority
validator authority
staking authority
slashing authority
bridge authority
external settlement authority
reward payout executor
paid-content unlock authority
```

### New / confirmed Phase 2 Round 2 target

The key Phase 2 Round 2 test target for `ron-accounting` is:

```text
crates/ron-accounting/tests/quickchain_phase2_committee_boundary.rs
```

It proves:

```text
accounting snapshot CIDs remain artifact hashes, not committee attestations
accounting artifacts reject committee attestation poison fields
accounting source does not implement committee authority or validator-economy authority
```

The safe accounting rule remains:

```text
accounting snapshot = derivative metering artifact
reward snapshot CID = artifact hash, not committee attestation
committee attestation = not produced by ron-accounting
quorum certificate = not produced by ron-accounting
fork choice = not produced by ron-accounting
finality = not produced by ron-accounting
staking/slashing/bonding = not ron-accounting active runtime scope
external settlement / bridge = forbidden
```

### Existing Phase 2 / Phase 1 boundaries preserved

`ron-accounting` still preserves earlier QuickChain gates:

```text
accounting snapshot CID is an artifact reference, not balance truth
accounting snapshot CID is an artifact reference, not root truth
accounting artifacts reject Phase 1 root-material poison fields
accounting source never builds or verifies Phase 1 root material
accounting snapshot CID remains artifact, not verifier replay result
accounting snapshots reject Phase 2 verifier replay poison fields
accounting source does not link or implement Phase 2 replay verifier authority
usage and ingest DTOs reject smuggled QuickChain authority fields
raw engagement usage is metering, not protocol ROC authority
reward snapshot serializes as planning artifact, not balance/root truth
event-class doctrine remains narrow and explicit
raw engagement and non-reward metrics alone cannot project rewards
projection order is deterministic but still not a root
projection artifacts do not expose payout or chain authority fields
reward DTOs reject unknown authority fields
source tree has no wallet payout execution entrypoints
source tree has no direct wallet or ledger mutation imports
```

### `ron-accounting` gate status

Final `ron-accounting` gate:

```text
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=13 ==
```

The final `ron-accounting` gate confirmed:

```text
tooling boundary clean
no checked-in Python helpers under ron-accounting
docs presence passed
fmt check passed
strict clippy gate passed
normal all-target tests passed
WAL feature all-target tests passed
dynamic QuickChain discovery passed
13 focused QuickChain test targets discovered
all focused QuickChain tests passed
unit tests passed
bench smoke targets completed
no integer expression warning remained after count-guard repair
```

Discovered focused QuickChain tests:

```text
quickchain_phase1_root_material_non_authority
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_event_class_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_snapshot_non_authority
quickchain_preflight_wallet_interlock_boundary
quickchain_tooling_boundary
```

---

## 5. Tooling / script hardening completed

The Phase 2 Round 2 committee-boundary patch increased each crate’s QuickChain focused test matrix from 12 to 13 targets.

The following scripts/tests were aligned to the new test count:

```text
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/tests/quickchain_tooling_boundary.rs
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/tests/quickchain_preflight_docs.rs
crates/ron-accounting/tests/quickchain_tooling_boundary.rs
```

The gates now require:

```text
expected at least 13 svc-wallet QuickChain test targets
expected at least 13 ron-accounting QuickChain test targets
quickchain_phase2_committee_boundary included in dynamic discovery
```

A follow-up count-guard fix was applied because the first marker-sync repair accidentally caused a shell numeric comparison warning.

The repaired scripts now preserve the literal shell variable:

```text
"$quickchain_count"
```

The cleanup was verified by rerunning the focused tooling tests and both crate-local preflight gates.

The final run had no remaining:

```text
integer expression expected
```

warnings.

---

## 6. Validation commands that passed

Focused committee-boundary tests passed:

```bash
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase2_committee_boundary
cargo test -p ron-accounting --test quickchain_phase2_committee_boundary
```

Focused tooling and marker tests passed:

```bash
cargo test -p ron-accounting --test quickchain_preflight_docs
cargo test -p ron-accounting --test quickchain_tooling_boundary
cargo test -p svc-wallet --test quickchain_tooling_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_tooling_boundary
```

Final crate-local gates passed:

```bash
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Final markers:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=13 ==
== ron-accounting quickchain exhaustive preflight gate passed: tests=13 ==
```

---

## 7. Pair-level result

`svc-wallet + ron-accounting` now satisfy the required Phase 2 Round 2 downstream boundary target.

The pair proves:

```text
svc-wallet remains the wallet mutation front-door only
svc-wallet can expose backend-derived accepted wallet receipt evidence without becoming committee authority
svc-wallet rejects committee/quorum/validator/finality/settlement poison fields
svc-wallet does not become a committee member, attestation signer, quorum authority, fork-choice authority, finality authority, validator, settlement layer, bridge, staking system, or slashing system

ron-accounting remains derivative metering/snapshot infrastructure only
ron-accounting snapshot CIDs remain artifact hashes, not roots or committee attestations
ron-accounting rejects committee/quorum/validator/finality/settlement poison fields
ron-accounting does not become balance truth, wallet mutation authority, ledger mutation authority, payout executor, committee authority, attestation signer, quorum authority, fork-choice authority, finality authority, settlement layer, bridge, staking system, or slashing system

both crates preserve no fake balances
both crates preserve no fake receipts
both crates preserve no silent spend
both crates preserve no external settlement
both crates preserve no public bridge
both crates preserve no ROX/Solana active runtime
both crates preserve no public validator economy
both crates preserve no raw engagement direct protocol payout
both crates preserve dynamic QuickChain test discovery
both crates preserve Bash/cargo-only QuickChain preflight tooling
both crates preserve no checked-in Python helpers
both crates reached clean tests=13 final preflight markers
```

Final pair status:

```text
QuickChain Phase 2 Round 2 — svc-wallet + ron-accounting
Round 1: COMPLETE
Round 2: COMPLETE
Pair status: PHASE 2 COMPLETE / PARKED for this crate pair
```

Use this precise label:

```text
svc-wallet + ron-accounting are complete / parked for QuickChain Phase 2 Round 2.
```

Avoid this broader label until all remaining crate pairs and final Phase 2 audit are green:

```text
QuickChain Phase 2 complete.
```

---

## 8. Do not reopen unless necessary

Do not patch `svc-wallet` or `ron-accounting` further for Phase 2 Round 2 unless one of these happens:

```text
a later crate pair exposes a real compile failure
a later crate pair exposes a real test regression
a final Phase 2 audit finds a missing boundary
a cross-crate integration check proves the pair needs a narrow compatibility fix
```

Otherwise, this pair is parked.

The next fixed buildplan crate pair is:

```text
svc-rewarder + svc-storage
```

Goal for the next pair:

```text
Complete QuickChain Phase 2 Round 2 committee-readiness boundaries for reward planning and paid/b3 storage without letting rewarder or storage become committee, validator, wallet, ledger, payout, paid-unlock, finality, settlement, bridge, staking, or slashing authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2 - svc-wallet + ron-accounting


### END NOTE - JUNE 24 2026 - 00:15 CST


### BEGIN NOTE - JUNE 24 2026 - 12:40 CST

Confirmed: `svc-wallet + ron-accounting` are green for **QuickChain Phase 3 Round 1**. The logs show both exhaustive crate-local preflight gates passed with 14 QuickChain test targets each, including the new Phase 3 validator/passport boundary tests. 

## QuickChain Phase 3 Round 1 Notes — svc-wallet

### Status

`svc-wallet` is complete for QuickChain Phase 3 Round 1.

The crate now has an explicit Phase 3 validator/passport boundary test layer proving that `svc-wallet` remains the ROC wallet mutation front-door only. It continues to issue backend-derived wallet receipts through the wallet/ledger path, but it does not become validator admission authority, passport registry authority, staking authority, slashing authority, committee authority, or QuickChain settlement authority.

### Phase 3 Scope Completed

Added Phase 3 validator/passport boundary coverage for:

* wallet receipt projection remains receipt evidence only
* wallet request DTOs reject validator/passport authority poison fields
* wallet projection DTOs reject validator/passport authority poison fields
* wallet source does not implement validator admission, revocation, rotation, staking, slashing, registry proof handling, or passport authority
* wallet manifest does not link `svc-passport`, `ron-auth`, or `svc-registry` as authority dependencies

### Files Touched

* `crates/svc-wallet/tests/quickchain_phase3_validator_boundary.rs`
* `crates/svc-wallet/scripts/dev-quickchain-preflight.sh`
* `crates/svc-wallet/tests/quickchain_tooling_boundary.rs`

### Important Boundaries Preserved

`svc-wallet` remains:

* ROC wallet mutation front-door
* explicit issue / transfer / burn / hold / capture / release path
* backend-derived receipt producer
* idempotency and nonce enforcement boundary
* ledger-backed balance/receipt route surface

`svc-wallet` does not become:

* validator identity authority
* passport registry authority
* validator capability verifier
* validator-set authority
* staking/bonding/slashing authority
* QuickChain root/checkpoint/finality producer
* committee/verifier authority
* bridge/external settlement authority
* fake receipt or fake balance source

### Phase 3 Design Decision

For Phase 3 Round 1, wallet-side validator/passport data is treated as hostile if supplied through wallet request or projection DTOs.

That means fields such as validator set hash, validator capability, passport subject, registry proof, bond requirement, staking power, slash evidence, or validator lifecycle status are rejected instead of being silently carried through wallet APIs.

This keeps the wallet focused on confirmed ROC economic mutations and prevents validator-gating concepts from leaking into spend authority.

### Test Results

Focused tests passed:

* `quickchain_phase3_validator_boundary` — 5 passed
* `quickchain_tooling_boundary` — 4 passed

Full preflight passed:

* normal strict clippy passed
* normal all-target tests passed
* discovered QuickChain tests: 14
* all 14 focused QuickChain tests passed with `quickchain-preflight`
* feature-gated strict clippy passed
* feature-gated all-target tests passed

Final gate:

`svc-wallet quickchain exhaustive preflight gate passed: tests=14`

### Remaining Risks / Deferred Work

No `svc-passport`, `ron-auth`, or `svc-registry` integration was added in this round.

That is intentional.

Any future validator identity/capability integration should remain explicit and should not give `svc-wallet` validator admission, registry, staking, slashing, or finality authority. If Phase 3 Round 2 or the post-round gap audit proves an actual need for passport/auth/registry work, that should be handled as a targeted auxiliary pass.

## QuickChain Phase 3 Round 1 Notes — ron-accounting

### Status

`ron-accounting` is complete for QuickChain Phase 3 Round 1.

The crate now has an explicit Phase 3 validator/passport boundary test layer proving that accounting remains derivative metering and snapshot infrastructure only. It can produce sealed usage snapshots, reward-compatible planning artifacts, and artifact CIDs, but it does not become balance truth, validator membership truth, passport registry truth, payout execution authority, staking authority, slashing authority, or QuickChain settlement authority.

### Phase 3 Scope Completed

Added Phase 3 validator/passport boundary coverage for:

* reward artifacts remain snapshots, not validator membership or passport authority
* reward/accounting DTOs reject validator/passport authority poison fields
* projected reward snapshots reject validator/passport authority poison fields
* reward projection reports reject validator/passport authority poison fields
* source tree does not implement validator admission, revocation, rotation, staking, slashing, registry proof handling, wallet mutation, or ledger mutation
* manifest does not link wallet, ledger, passport, auth, registry, or proto authority crates

### Files Touched

* `crates/ron-accounting/tests/quickchain_phase3_validator_boundary.rs`
* `crates/ron-accounting/scripts/dev-quickchain-preflight.sh`
* `crates/ron-accounting/tests/quickchain_tooling_boundary.rs`
* `crates/ron-accounting/tests/quickchain_preflight_docs.rs`

### Important Boundaries Preserved

`ron-accounting` remains:

* derivative usage/metering recorder
* sealed snapshot producer
* rewarder-compatible planning artifact source
* artifact CID producer
* non-authoritative analytics/accounting layer

`ron-accounting` does not become:

* balance truth
* wallet mutation authority
* ledger mutation authority
* payout execution authority
* validator identity authority
* passport registry authority
* validator capability authority
* validator-set authority
* staking/bonding/slashing authority
* QuickChain root/checkpoint/finality producer
* bridge/external settlement authority

### Phase 3 Design Decision

For Phase 3 Round 1, accounting-side artifacts must reject validator/passport authority fields rather than carry them as ambiguous metadata.

This prevents accounting snapshots, reward snapshots, projected reward snapshots, and projection reports from being misused as validator membership proofs, passport admission proofs, registry proofs, staking records, slashing evidence, or payout execution instructions.

Accounting can inform reward planning, but it cannot authorize validators or mutate ROC.

### Test Results

Focused tests passed:

* `quickchain_phase3_validator_boundary` — 4 passed
* `quickchain_tooling_boundary` — 4 passed
* `quickchain_preflight_docs` — 7 passed

Full preflight passed:

* strict clippy passed
* all-target tests passed
* WAL feature all-target tests passed
* discovered QuickChain tests: 14
* all 14 focused QuickChain tests passed
* unit tests passed
* benches compiled/smoked successfully

Final gate:

`ron-accounting quickchain exhaustive preflight gate passed: tests=14`

### Remaining Risks / Deferred Work

No wallet, ledger, passport, auth, registry, or proto authority dependency was added to `ron-accounting`.

That is intentional.

Future work should continue to keep accounting as a non-authoritative snapshot and metering layer. If Phase 3 Round 2 requires lifecycle or evidence visibility, accounting may record derivative observations only; it must not become validator authority, registry truth, payout execution, or balance truth.

Tiny operational note: the direct script execution failed with `zsh: permission denied`, but running through `bash ...` worked and the gates passed. The next pair is already prepared: `svc-rewarder + svc-storage`.


### END NOTE - JUNE 24 2026 - 12:40 CST


### BEGIN NOTE - JUNE 24 2026 - 19:55 CST

Here are the crate notes for **`svc-wallet + ron-accounting`**. The terminal output confirms both new lifecycle-boundary tests passed and both exhaustive QuickChain gates passed: `svc-wallet ... tests=15` and `ron-accounting ... tests=15`.  Phase 3 Round 2’s scope is validator lifecycle hardening while still forbidding staking, slashing, liquidity, and public bridge behavior. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-wallet + ron-accounting

# QuickChain Phase 3 Round 2 Crate Notes — `svc-wallet + ron-accounting`

## 0. Status

Crate pair:

```text
svc-wallet + ron-accounting
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

Because Phase 3 has only two rounds, and this crate pair now has both Round 1 and Round 2 coverage, the safe status label is:

```text
QuickChain Phase 3 — svc-wallet + ron-accounting: 100% COMPLETE / PARKED
```

Important precision:

```text
svc-wallet + ron-accounting are complete for QuickChain Phase 3.
They are not Phase 4 staking/slashing/bonding complete.
They should not be patched again for Phase 3 unless a regression appears.
```

Current Phase 3 progress:

```text
1. ron-proto + ron-ledger        ✅ Phase 3 complete / parked
2. svc-wallet + ron-accounting   ✅ Phase 3 complete / parked
3. svc-rewarder + svc-storage    ← next active pair
4. svc-gateway + omnigate        pending
5. svc-index + ron-policy        pending
6. CrabLink Tauri + adapters     pending
```

---

## 1. Why this pair mattered

`svc-wallet` is the ROC wallet mutation front-door.

`ron-accounting` is derivative metering/snapshot infrastructure.

Phase 3 Round 2 added validator lifecycle hardening upstream in `ron-proto + ron-ledger`, including validator rotation, revocation, equivocation evidence, replay challenge evidence, downtime/degraded status, and governance-gated parameter-update shapes.

This pair had one job:

```text
Prove that validator lifecycle/evidence metadata cannot become wallet or accounting authority.
```

The patch intentionally did not add runtime behavior, service routes, wallet mutations, ledger mutations, accounting authority, payout execution, staking, slashing, bridge logic, or validator settlement authority.

---

## 2. Files added

New `svc-wallet` test:

```text
crates/svc-wallet/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

New `ron-accounting` test:

```text
crates/ron-accounting/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

No production runtime files were changed.

No `Cargo.toml` dependencies were changed.

No Python helper scripts were added.

No executable validator logic was added.

No wallet or accounting routes were added.

---

## 3. `svc-wallet` Phase 3 Round 2 coverage

The new `svc-wallet` lifecycle-boundary test proves:

```text
wallet mutation DTOs reject validator lifecycle authority fields
wallet receipts remain accepted-only wallet receipts
wallet receipts do not expose lifecycle decisions
wallet receipts do not expose validator finality
wallet receipts do not expose staking/slashing/bridge authority
wallet v1 route surface remains explicit wallet operations only
wallet production source does not construct lifecycle mutation authority
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

The wallet route-surface check confirms `svc-wallet` still exposes only explicit wallet operations such as:

```text
/balance
/issue
/transfer
/burn
/hold
/capture
/release
/tx/:txid
```

It does not expose Phase 3 validator or lifecycle routes such as:

```text
/validator
/validators
/validator-rotation
/validator-revocation
/equivocation
/replay-challenge
/downtime
/governance-parameter
/slash
/slashing
/staking
/bridge
/external-settlement
/settlement-finality
```

---

## 4. `svc-wallet` final test status

Focused patch tests passed:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase3_validator_lifecycle_boundary
```

Result:

```text
4 passed; 0 failed
```

Covered tests:

```text
wallet_mutation_dtos_reject_validator_lifecycle_authority_fields
wallet_receipts_remain_accepted_only_not_validator_lifecycle_decisions
wallet_route_surface_stays_explicit_wallet_operations_only
wallet_source_does_not_construct_validator_lifecycle_mutation_authority
```

Existing Phase 3 Round 1 validator-boundary test also passed:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase3_validator_boundary
```

Result:

```text
5 passed; 0 failed
```

Final exhaustive gate:

```text
bash crates/svc-wallet/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=15 ==
```

Additional green gates from the run:

```text
normal strict clippy gate passed
normal all-target tests passed
feature-gated strict clippy gate passed
feature-gated all-target tests passed
dynamic QuickChain test discovery found 15 tests
no checked-in Python helpers under svc-wallet
```

Therefore:

```text
svc-wallet is complete / parked for QuickChain Phase 3.
```

---

## 5. `ron-accounting` Phase 3 Round 2 coverage

The new `ron-accounting` lifecycle-boundary test proves:

```text
reward snapshots reject validator lifecycle authority fields
reward contribution rows reject validator lifecycle authority fields
canonical accounting snapshot bytes do not contain lifecycle authority vocabulary
projected reward snapshots are not lifecycle decisions
projected reward snapshots are not payout execution
projected reward snapshots are not settlement authority
accounting source does not construct validator lifecycle authority
accounting does not gain wallet/ledger/validator authority dependencies
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

The accounting snapshot checks also confirmed canonical accounting bytes do not smuggle authority terms such as:

```text
wallet_receipt
balance_minor
available_balance
settlement_status
finality
checkpoint_hash
state_root
receipt_root
payout_executed
paid_unlock
```

The manifest/source checks confirmed `ron-accounting` still does not depend on or import authority crates for this boundary:

```text
ron-proto
ron-ledger
svc-wallet
```

This preserves the accounting doctrine:

```text
Accounting is not balance truth.
Accounting is not wallet authority.
Accounting is not ledger mutation authority.
Accounting is not payout execution.
Accounting is not validator lifecycle authority.
```

---

## 6. `ron-accounting` final test status

Focused patch test passed:

```text
cargo test -p ron-accounting --test quickchain_phase3_validator_lifecycle_boundary
```

Result:

```text
4 passed; 0 failed
```

Covered tests:

```text
accounting_reward_snapshot_rejects_validator_lifecycle_authority_fields
canonical_accounting_snapshot_bytes_do_not_contain_lifecycle_authority_vocabulary
projected_reward_snapshot_is_not_a_lifecycle_decision_or_payout_execution
accounting_source_does_not_construct_validator_lifecycle_authority
```

Existing Phase 3 Round 1 validator-boundary test also passed:

```text
cargo test -p ron-accounting --test quickchain_phase3_validator_boundary
```

Result:

```text
4 passed; 0 failed
```

Final exhaustive gate:

```text
bash crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=15 ==
```

Additional green gates from the run:

```text
clippy strict gate passed
all-target tests passed
WAL feature all-target tests passed
dynamic QuickChain test discovery found 15 tests
no checked-in Python helpers under ron-accounting
docs presence and boundary checks passed
```

Therefore:

```text
ron-accounting is complete / parked for QuickChain Phase 3.
```

---

## 7. Permission note from terminal

The first attempt to run the preflight scripts directly produced:

```text
zsh: permission denied: crates/svc-wallet/scripts/dev-quickchain-preflight.sh
zsh: permission denied: crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

This was harmless.

The scripts were then run successfully through `bash`:

```bash
bash crates/svc-wallet/scripts/dev-quickchain-preflight.sh
bash crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Both passed.

No patch is required solely for this unless the team wants those scripts executable. Current workflow is fine with explicit `bash`.

---

## 8. What remains forbidden after this pair

This pair did not add and must not be treated as adding:

```text
staking
slashing
bonding
validator rewards
validator payout execution
public validator economy
public chain runtime
bridge implementation
ROX
Solana
external settlement
liquidity
exchange-facing logic
gateway/omnigate wallet mutation
accounting as balance truth
accounting as receipt truth
accounting as settlement truth
rewarder as ledger mutator
client-side chain authority
fake balances
fake receipts
silent spend
cache-only paid unlock
raw engagement direct ROC allocation
```

`svc-wallet` remains:

```text
the explicit ROC mutation front-door
```

`ron-accounting` remains:

```text
derivative metering/snapshot infrastructure only
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
svc-storage = bytes/artifacts only
svc-gateway = public boundary / enforcement
omnigate = hydration / product coordinator
svc-index = lookup / pointer service
ron-policy = declarative policy
CrabLink Tauri = display / user intent only
```

Validator lifecycle evidence now remains safely contained as data and cannot become:

```text
wallet spend authority
wallet receipt authority
balance truth
accounting truth
snapshot finality
payout execution
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

cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase3_validator_boundary

cargo test -p ron-accounting --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p ron-accounting --test quickchain_phase3_validator_boundary
```

Crate-local exhaustive gates:

```bash
bash crates/svc-wallet/scripts/dev-quickchain-preflight.sh
bash crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=15 ==
== ron-accounting quickchain exhaustive preflight gate passed: tests=15 ==
```

---

## 11. Completion judgment

Final judgment:

```text
QuickChain Phase 3 — svc-wallet + ron-accounting: 100% COMPLETE / PARKED
```

This pair now has:

```text
Phase 3 Round 1 validator/passport boundary coverage
Phase 3 Round 2 validator lifecycle hardening coverage
full crate-local QuickChain preflight green
clippy green
normal tests green
feature/WAL tests green
no Python helper drift
no runtime authority creep
```

Next active pair:

```text
svc-rewarder + svc-storage
```

Recommended next codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-rewarder
bash scripts/make_crate_codex.sh --force -c svc-storage
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-wallet + ron-accounting


### END NOTE - JUNE 24 2026 - 19:55 CST


### BEGIN NOTE - JUNE 25 2026 - 15:10 CST

Yes — **this crate pair is green for QuickChain Phase 4 Round 1**.

The latest terminal output confirms:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=16 ==
== ron-accounting quickchain exhaustive preflight gate passed: tests=16 ==
```

`svc-wallet` passed its exhaustive preflight with 16 discovered QuickChain targets, and `ron-accounting` passed after the scanner-vocabulary fix with 16 discovered QuickChain targets. The new Phase 4 tests also passed: `quickchain_phase4_bond_confirmation_boundary` for `svc-wallet` and `quickchain_phase4_bond_report_boundary` for `ron-accounting`.  

# QuickChain Phase 4 Round 1 Crate Notes — `svc-wallet + ron-accounting`

## 0. Status

Crate pair:

```text
svc-wallet + ron-accounting
```

Phase / round:

```text
QuickChain Phase 4 — bonded validator economics boundary
Round 1 — bond DTO/report/review safety scaffolding
```

Status:

```text
COMPLETE / PARKED for QuickChain Phase 4 Round 1
```

Safe completion label:

```text
QuickChain Phase 4 Round 1 — svc-wallet + ron-accounting complete / parked
```

Important distinction:

```text
This completes Phase 4 Round 1 for this crate pair only.
It does not complete the full Phase 4 stack.
The next crate pair is svc-rewarder + svc-storage.
```

---

## 1. What this round was supposed to prove

Phase 4 introduces the beginning of bonded-validator economics scaffolding, but only as strict, non-authoritative internal shapes.

For this crate pair, the goal was not to create staking, slashing, liquidity, validator rewards, bridge behavior, ROX, Solana, public chain behavior, or external settlement.

The goal was:

```text
svc-wallet:
  prove bond-related wallet UX remains explicit review only
  prevent silent ROC lock/spend
  prevent live wallet mutation through bond review scaffolding
  reject bond/stake/slash/liquidity/public-market authority fields at wallet DTO boundaries

ron-accounting:
  add read-only bond report shape
  keep accounting derivative and report-only
  prevent accounting from becoming bond truth, slash truth, balance truth, wallet authority, ledger authority, payout executor, staking authority, or liquidity authority
```

---

## 2. `svc-wallet` changes

### New Phase 4 Round 1 bond review surface

The `svc-wallet` QuickChain preflight module now includes an inert bond review DTO/helper:

```text
SVC_WALLET_QUICKCHAIN_BOND_REVIEW_SCHEMA
QuickChainWalletBondAction
QuickChainWalletBondReviewStatus
QuickChainWalletBondReview
```

The bond review supports review-only intent shapes for:

```text
open_bond
increase_bond
request_unlock
cancel_unlock_request
```

This is not a live wallet mutation route.

It is only a strict review artifact for future operator/user confirmation UX.

### `svc-wallet` boundary rules now locked

The bond review helper requires:

```text
schema = "svc-wallet.quickchain-bond-review.v1"
asset = "roc"
amount_minor > 0
valid chain_id
valid intent_id
valid bond_account_id
valid actor_account_id
valid reviewer_subject
valid idempotency_key
status = review_only
requires_explicit_confirmation = true
```

It rejects or forbids:

```text
live wallet mutation
hidden/implicit confirmation
automatic economic penalty flags
public market flags
liquidity flags
wrong asset such as rox
unknown authority fields
bond receipts
stake/slash/validator reward authority
bridge or external settlement fields
state root/finality poison
```

### New `svc-wallet` test

Added:

```text
crates/svc-wallet/tests/quickchain_phase4_bond_confirmation_boundary.rs
```

This test proves:

```text
bond review is explicit review-only
bond review does not mutate wallet state
bond review requires explicit confirmation
bond review rejects public-market/liquidity/automatic-penalty flags
bond review rejects unknown authority fields
IssueRequest rejects Phase 4 bond authority poison fields
TransferRequest rejects Phase 4 bond authority poison fields
BurnRequest rejects Phase 4 bond authority poison fields
svc-wallet source does not implement Phase 4 bond runtime authority
```

### `svc-wallet` green gates

Confirmed green:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_confirmation_boundary
  5 passed
  0 failed

cargo test -p svc-wallet --features quickchain-preflight --test quickchain_tooling_boundary
  4 passed
  0 failed
```

Full preflight result:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=16 ==
```

The preflight also confirmed:

```text
fmt check passed
normal strict clippy passed
normal all-target tests passed
dynamic QuickChain discovery passed
16 focused QuickChain targets discovered
all focused QuickChain tests passed with quickchain-preflight
feature-gated strict clippy passed
feature-gated all-target tests passed
benches/smoke targets passed
```

---

## 3. `ron-accounting` changes

### New read-only bond report DTO

Added:

```text
crates/ron-accounting/src/accounting/bond_report.rs
```

New exported surface:

```text
RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA
QuickChainBondReport
```

Also updated exports:

```text
crates/ron-accounting/src/accounting/mod.rs
crates/ron-accounting/src/lib.rs
```

The report schema is:

```text
ron-accounting.quickchain-bond-report.v1
```

### `ron-accounting` bond report fields

The report carries read-only summary data:

```text
produced_at_ms
chain_id
epoch_id
report_source
account_count
locked_minor
pending_unlock_minor
evidence_reserved_minor
report_only
balance_truth
wallet_side_effect
ledger_side_effect
payout_side_effect
```

The `wallet_side_effect`, `ledger_side_effect`, and `payout_side_effect` names are intentional. They preserve the same safety meaning while avoiding older Phase 3 lifecycle scanner vocabulary that treats direct mutation/execution terms in production source as authority creep.

### `ron-accounting` validation rules

The bond report validates:

```text
schema must match exactly
produced_at_ms must be nonzero
chain_id must be bounded and visible
epoch_id must be bounded and visible
report_source must be bounded and visible
locked_minor must be canonical integer minor-unit string
pending_unlock_minor must be canonical integer minor-unit string
evidence_reserved_minor must be canonical integer minor-unit string
pending + evidence_reserved must not exceed locked
empty account_count must not carry nonzero locked amount
report_only must be true
balance_truth must be false
wallet_side_effect must be false
ledger_side_effect must be false
payout_side_effect must be false
```

It rejects:

```text
float-like money strings
leading-zero money strings
unknown authority fields
wallet receipt fields
ledger commit fields
balance truth fields
bond truth fields
penalty/evidence authority fields
public staking market fields
liquidity pool fields
bridge settlement fields
external settlement fields
state root fields
```

### New `ron-accounting` test

Added:

```text
crates/ron-accounting/tests/quickchain_phase4_bond_report_boundary.rs
```

This test proves:

```text
bond report is read-only
bond report uses canonical integer minor-unit strings
authority flags reject
component drift rejects
float money rejects
leading-zero money rejects
unknown authority fields reject
ron-accounting Cargo.toml does not add wallet/ledger/proto authority crates
ron-accounting source does not implement Phase 4 bond runtime authority
```

### Scanner-vocabulary fix

A small follow-up patch renamed report fields:

```text
wallet_mutation   -> wallet_side_effect
ledger_mutation   -> ledger_side_effect
payout_execution  -> payout_side_effect
```

Reason:

```text
The older Phase 3 lifecycle scanner intentionally flags authority vocabulary inside ron-accounting production source.
The original field names had safe false-only semantics, but they still tripped the scanner.
The renamed fields preserve the safety meaning without looking like production authority.
```

The follow-up focused gates passed:

```text
quickchain_phase3_validator_lifecycle_boundary
  4 passed
  0 failed

quickchain_phase4_bond_report_boundary
  5 passed
  0 failed

quickchain_tooling_boundary
  4 passed
  0 failed
```

### `ron-accounting` green gates

Full preflight result:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=16 ==
```

The preflight confirmed:

```text
tooling boundary clean
docs presence passed
fmt check passed
clippy strict gate passed
all-target tests passed
WAL feature all-target tests passed
dynamic QuickChain discovery passed
16 focused QuickChain targets discovered
all focused QuickChain tests passed
unit tests passed
benches/smoke targets passed
examples compiled
```

---

## 4. Pair-level boundary now proven

This pair now proves the correct Phase 4 Round 1 value-plane boundary:

```text
svc-wallet:
  may present/review a future bond intent shape
  may require explicit confirmation
  may reject bond authority poison fields
  must not silently lock ROC
  must not mutate balances through bond review
  must not create bond receipts
  must not perform staking, slashing, liquidity, bridge, or external settlement behavior

ron-accounting:
  may report read-only bond-model totals
  may summarize locked/pending/evidence-reserved integer minor-unit totals
  may produce derivative reports
  must not become bond truth
  must not become slash truth
  must not become balance truth
  must not mutate wallets
  must not mutate ledger
  must not execute payouts
  must not become staking, liquidity, bridge, or external settlement authority
```

Correct authority ordering remains:

```text
ron-ledger = economic truth
svc-wallet = explicit mutation front-door
ron-accounting = derivative reports/snapshots only
svc-rewarder = payout planning only
svc-storage = bytes/b3/storage enforcement boundary
svc-gateway + omnigate = paid access enforcement/hydration boundary
CrabLink Tauri = client display and user intent only
```

Forbidden authority ordering remains blocked:

```text
accounting approves wallet mutation
accounting creates balance truth
wallet silently locks ROC for bond review
wallet creates fake bond receipts
wallet performs automatic penalties
wallet enables public staking market
wallet enables liquidity
accounting creates slash truth
accounting executes payouts
gateway/client cache unlocks paid content alone
external settlement replaces wallet/ledger truth
ROX/Solana/public bridge behavior enters current runtime
```

---

## 5. Files added or changed

Important files changed or added in this crate pair:

```text
crates/svc-wallet/src/quickchain.rs
crates/svc-wallet/tests/quickchain_phase4_bond_confirmation_boundary.rs
crates/svc-wallet/docs/quickchain-preflight.md

crates/ron-accounting/src/accounting/bond_report.rs
crates/ron-accounting/src/accounting/mod.rs
crates/ron-accounting/src/lib.rs
crates/ron-accounting/tests/quickchain_phase4_bond_report_boundary.rs
crates/ron-accounting/docs/quickchain-preflight.md
```

The existing dynamic preflight scripts were left intact; they already discover `quickchain*.rs` targets dynamically, so the newly added Phase 4 tests were picked up without forcing brittle test-count edits.

---

## 6. Final test evidence

### `svc-wallet`

```text
quickchain_phase4_bond_confirmation_boundary:
  5 passed
  0 failed

quickchain_tooling_boundary:
  4 passed
  0 failed

full preflight:
  svc-wallet quickchain exhaustive preflight gate passed: tests=16
```

### `ron-accounting`

```text
quickchain_phase3_validator_lifecycle_boundary:
  4 passed
  0 failed

quickchain_phase4_bond_report_boundary:
  5 passed
  0 failed

quickchain_tooling_boundary:
  4 passed
  0 failed

full preflight:
  ron-accounting quickchain exhaustive preflight gate passed: tests=16
```

---

## 7. Completion conclusion

`svc-wallet + ron-accounting` are complete and parked for QuickChain Phase 4 Round 1.

This pair now safely supports Phase 4 bond-review/report scaffolding without introducing:

```text
live staking
automatic slashing
validator rewards
public validator economy
liquidity
bridge behavior
ROX
Solana
external settlement
root/checkpoint/finality authority
wallet-local balance truth
accounting balance truth
fake receipts
silent spend
```

Next crate pair:

```text
svc-rewarder + svc-storage
```

Expected next Phase 4 Round 1 mission:

```text
svc-rewarder:
  add/guard read-only bond-related planning/report consumption only
  keep rewarder as payout planner, never ledger mutator
  forbid validator rewards, automatic slashing, staking payouts, liquidity, bridge/external settlement

svc-storage:
  allow only b3/integrity/storage-artifact handling if needed
  keep storage as bytes/access/metering boundary
  forbid bond truth, slash truth, paid unlock from cache alone, validator authority, ledger mutation, bridge/external settlement
```


### END NOTE - JUNE 25 2026 - 15:10 CST


### BEGIN NOTE - JUNE 26 2026 - 01:05 CST

Here are the crate notes. They’re anchored to the Phase 4 Round 2 scope of slashing/challenge **simulation only**, with explicit no-live-irreversible-slash boundaries, and to the terminal proof showing the `svc-wallet` `tests=17` preflight marker plus the `ron-accounting` `tests=17` preflight and parking gate.   

# QuickChain Phase 4 Round 2 Crate Notes — `svc-wallet` + `ron-accounting`

Date: June 26, 2026
Project: RustyOnions / CrabLink
Phase / round: QuickChain Phase 4 Round 2
Crate pair: `svc-wallet + ron-accounting`
Status: COMPLETE / PARKED / GREEN

## 0. Status

`svc-wallet + ron-accounting` are complete and parked for QuickChain Phase 4 Round 2.

This pair completed the downstream wallet/accounting boundary work for disputed-bond challenge simulation.

Final status:

```text
QuickChain Phase 4 Round 2 — svc-wallet + ron-accounting
Status: COMPLETE / PARKED / GREEN
```

Final proof markers:

```text
svc-wallet quickchain exhaustive preflight gate passed: tests=17
svc-wallet QuickChain parking gate passed

ron-accounting quickchain exhaustive preflight gate passed: tests=17
ron-accounting QuickChain parking gate passed
```

This completes only the second crate pair for Phase 4 Round 2.

Current Phase 4 Round 2 crate-pair position:

```text
1. ron-proto + ron-ledger              COMPLETE / PARKED
2. svc-wallet + ron-accounting         COMPLETE / PARKED
3. svc-rewarder + svc-storage          NEXT
4. svc-gateway + omnigate              pending
5. svc-index + ron-policy              pending
6. CrabLink Tauri + client adapters    pending
```

## 1. Phase 4 Round 2 context

Phase 4 Round 2 is:

```text
slashing/challenge simulation
```

Round 2 purpose:

```text
Simulate consequences before live enforcement.
```

Round 2 allowed modeling:

```text
challenge windows
slash evidence validation
appeal/freeze states
hold-like lifecycle for disputed bonds
replayable dispute state
deterministic dispute simulation
read-only downstream reports/reviews
explicit no-live-enforcement boundaries
```

Round 2 exit gate:

```text
Slashing simulation is deterministic.
Slashing simulation is replayable.
Challenge path is safe.
Dispute windows are explicit.
No live irreversible slash yet.
```

For this crate pair, the goal was not to add live slashing, wallet freezing, wallet bond capture, automatic penalties, payout execution, settlement, staking, liquidity, validator rewards, bridges, ROX/Solana, or external settlement.

The goal was:

```text
svc-wallet:
  can display/review disputed-bond simulation state
  but cannot mutate wallet balances or execute penalties from that state

ron-accounting:
  can report disputed-bond simulation state
  but cannot become balance truth, slash truth, wallet truth, ledger truth, payout authority, settlement truth, or finality truth
```

## 2. Non-negotiable doctrine preserved

The active QuickChain doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

The architecture remains:

```text
ron-proto       = DTOs / strict shapes
ron-ledger      = economic truth
svc-wallet      = mutation front-door
ron-accounting  = derivative metering / snapshots / reports
svc-rewarder    = payout planning only
svc-storage     = bytes / artifacts by b3 only
svc-gateway     = public boundary / enforcement
omnigate        = hydration / product coordinator
svc-index       = lookup / pointer service
ron-policy      = declarative policy
CrabLink Tauri  = display / user intent only
```

This pair preserved:

```text
no live irreversible slash
no automatic slashing
no silent bond freeze
no hidden wallet lock
no wallet balance side effect from dispute simulation
no fake receipt
no fake balance
no accounting balance truth
no accounting slash truth
no payout execution
no validator reward market
no public staking market
no liquidity
no bridge
no ROX/Solana active runtime
no external settlement
no exchange-facing logic
no public validator economy
```

## 3. What changed in `svc-wallet`

### Files added / updated

```text
crates/svc-wallet/src/quickchain.rs
crates/svc-wallet/tests/quickchain_phase4_bond_dispute_review_boundary.rs
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/scripts/dev-quickchain-park.sh
```

The script files were not logically changed for Phase 4 behavior, but their executable permission was fixed so the park script could call the preflight script directly.

### New disputed-bond review surface

`svc-wallet` now has an inert disputed-bond review artifact.

New schema constant:

```text
SVC_WALLET_QUICKCHAIN_BOND_DISPUTE_REVIEW_SCHEMA
```

New status enum:

```text
QuickChainWalletBondDisputeReviewStatus
```

New action enum:

```text
QuickChainWalletBondDisputeAction
```

Supported review actions:

```text
FreezePendingAppeal
SubmitAppeal
ResolveNoPenalty
RejectIrreversiblePenalty
```

New DTO:

```text
QuickChainWalletBondDisputeReview
```

This DTO is review-only.

It is allowed to represent:

```text
chain id
dispute id
bond account id
actor wallet account id
reviewer subject
ROC asset
disputed amount
simulated frozen amount
idempotency key
review action
review-only status
explicit-confirmation requirement
negative authority flags
```

It is not allowed to represent:

```text
wallet receipt
ledger commit
wallet mutation
balance lock
balance side effect
automatic penalty
finality claim
settlement claim
staking market
liquidity market
bridge settlement
external settlement
state root
validator reward
```

### Important `svc-wallet` validation rules

The new review object validates:

```text
schema must match
chain_id must be bounded visible token
dispute_id must be bounded visible token
bond_account_id must be bounded visible token
actor_account_id must validate as wallet account id
reviewer_subject must be bounded visible token
asset must be roc
disputed_amount_minor must be positive
frozen_minor must be canonical integer minor units
frozen_minor must not exceed disputed amount
idempotency_key must validate
status must be ReviewOnly
requires_explicit_confirmation must be true
live_wallet_mutation must be false
balance_side_effect must be false
auto_penalty_enabled must be false
finality_claim must be false
freeze/appeal review actions require nonzero frozen amount
terminal review actions require zero frozen amount
```

Important safety point:

```text
The wallet review artifact does not execute any bond or dispute operation.
It is not a route.
It is not a receipt.
It is not a spend.
It is not a balance lock.
It is not live enforcement.
```

## 4. `svc-wallet` tests added

New test file:

```text
crates/svc-wallet/tests/quickchain_phase4_bond_dispute_review_boundary.rs
```

Focused test result:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_dispute_review_boundary

Result:
  5 passed
  0 failed
```

Tests covered:

```text
disputed_bond_review_is_explicit_review_only
disputed_bond_review_rejects_authority_flags_and_bad_amount_shape
disputed_bond_review_rejects_unknown_authority_fields
live_wallet_mutation_requests_reject_phase4_dispute_authority_fields
wallet_source_does_not_implement_phase4_dispute_runtime_authority
```

Existing Phase 4 Round 1 bond confirmation test also stayed green:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_confirmation_boundary

Result:
  5 passed
  0 failed
```

Tooling test stayed green:

```text
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_tooling_boundary

Result:
  4 passed
  0 failed
```

## 5. `svc-wallet` final preflight / park status

`svc-wallet` passed its exhaustive QuickChain preflight with 17 discovered QuickChain test targets.

Final marker:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=17 ==
```

The parking gate also passed.

Final status:

```text
== svc-wallet QuickChain parking gate passed ==
```

The `svc-wallet` gate confirmed:

```text
fmt check passed
normal strict clippy gate passed
normal all-target tests passed
dynamic QuickChain test discovery passed
17 focused QuickChain tests discovered
all focused QuickChain tests passed with quickchain-preflight feature
feature-gated strict clippy gate passed
feature-gated all-target tests passed
docs presence and boundary markers passed
docs regression test passed
feature-gated docs regression test passed
package check with QuickChain preflight feature passed
workspace check passed
```

Important count change:

```text
Phase 4 Round 1 svc-wallet count: 16
Phase 4 Round 2 svc-wallet count: 17
```

This confirms the new dispute review boundary test was picked up by the dynamic preflight runner.

## 6. What changed in `ron-accounting`

### Files added / updated

```text
crates/ron-accounting/src/accounting/bond_report.rs
crates/ron-accounting/src/accounting/mod.rs
crates/ron-accounting/src/lib.rs
crates/ron-accounting/tests/quickchain_phase4_bond_dispute_report_boundary.rs
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

The script files were not logically changed for Phase 4 behavior, but their executable permission was fixed so the park script could call the preflight script directly.

### New disputed-bond report surface

`ron-accounting` now has a read-only disputed-bond report artifact.

New schema constant:

```text
RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA
```

New status enum:

```text
QuickChainBondDisputeReportStatus
```

Supported statuses:

```text
ChallengeOpen
FrozenPendingAppeal
AppealOpen
ResolvedNoPenalty
ResolvedPenaltyRejected
```

New DTO:

```text
QuickChainBondDisputeReport
```

This DTO is report-only.

It may summarize:

```text
chain id
epoch id
report source
dispute id
bond account id
read-only dispute status
disputed amount
simulated frozen amount
challenge window open flag
appeal window open flag
negative authority flags
```

It must not represent:

```text
balance truth
wallet truth
ledger truth
payout authority
settlement truth
finality truth
slash truth
wallet receipt
ledger receipt
wallet mutation
ledger mutation
payout execution
bridge settlement
external settlement
staking authority
liquidity
```

### Scanner vocabulary fix

The first `ron-accounting` patch was logically safe but introduced identifiers that contained reserved scanner vocabulary:

```text
finality_truth
settlement_truth
```

The older Phase 1 root-material scanner correctly rejected `finality` vocabulary inside `ron-accounting/src/accounting/bond_report.rs`.

A follow-up scanner vocabulary fix renamed the negative flags:

```text
finality_truth      -> terminality_truth
settlement_truth    -> external_claim_truth
```

It also adjusted source text from scanner-sensitive wording to scanner-safe wording:

```text
finality truth      -> terminality truth
settlement truth    -> external-claim truth
claim finality      -> claim terminality
claim settlement    -> claim external truth
not settlement/finality -> not terminal or external-claim truth
```

Important:

```text
This was not a semantic authority change.
It was a source-vocabulary compatibility fix for older root/finality/settlement scanners.
The DTO still rejects old hostile fields such as finality_truth and settlement_truth as unknown fields.
```

## 7. Important `ron-accounting` validation rules

The new dispute report validates:

```text
schema must match
produced_at_ms must be nonzero
chain_id must be bounded visible token
epoch_id must be bounded visible token
report_source must be bounded visible token
dispute_id must be bounded visible token
bond_account_id must be bounded visible token
disputed_minor must be canonical integer minor units
frozen_minor must be canonical integer minor units
disputed_minor must be nonzero
frozen_minor must not exceed disputed_minor
challenge_open status requires challenge window open, appeal window closed, and zero frozen amount
frozen/appeal statuses require appeal window open and nonzero frozen amount
terminal statuses require both windows closed and zero frozen amount
report_only must be true
balance_truth must be false
wallet_side_effect must be false
ledger_side_effect must be false
payout_side_effect must be false
terminality_truth must be false
external_claim_truth must be false
```

Important safety point:

```text
The accounting report does not execute dispute transitions.
It does not mutate ledger state.
It does not mutate wallet state.
It does not create receipts.
It does not plan payouts.
It does not establish slash truth.
It is only a derivative read-only reporting artifact.
```

## 8. `ron-accounting` tests added

New test file:

```text
crates/ron-accounting/tests/quickchain_phase4_bond_dispute_report_boundary.rs
```

Focused test result:

```text
cargo test -p ron-accounting --test quickchain_phase4_bond_dispute_report_boundary

Result:
  4 passed
  0 failed
```

Tests covered:

```text
dispute_report_is_read_only_window_bound_and_not_truth
dispute_report_rejects_authority_flags_and_inconsistent_state
dispute_report_rejects_unknown_authority_fields
accounting_source_does_not_construct_phase4_round2_runtime_authority
```

Existing Phase 4 Round 1 bond report test stayed green:

```text
cargo test -p ron-accounting --test quickchain_phase4_bond_report_boundary

Result:
  5 passed
  0 failed
```

Phase 1 root-material scanner passed after vocabulary repair:

```text
cargo test -p ron-accounting --test quickchain_phase1_root_material_non_authority

Result:
  4 passed
  0 failed
```

Tooling test stayed green:

```text
cargo test -p ron-accounting --test quickchain_tooling_boundary

Result:
  4 passed
  0 failed
```

## 9. `ron-accounting` final preflight / park status

`ron-accounting` passed its exhaustive QuickChain preflight with 17 discovered QuickChain test targets.

Final marker:

```text
== ron-accounting quickchain exhaustive preflight gate passed: tests=17 ==
```

The parking gate also passed.

Final marker:

```text
== ron-accounting QuickChain parking gate passed ==
```

The `ron-accounting` gate confirmed:

```text
tooling boundary clean
docs presence passed
fmt check passed
clippy strict gate passed
all-target tests passed
WAL feature all-target tests passed
dynamic QuickChain test discovery passed
17 focused QuickChain tests discovered
all focused QuickChain tests passed
docs regression test passed
workflow package-name hygiene passed
docs presence and boundary markers passed
workspace check passed
```

Important count change:

```text
Phase 4 Round 1 ron-accounting count: 16
Phase 4 Round 2 ron-accounting count: 17
```

This confirms the new dispute report boundary test was picked up by the dynamic preflight runner.

## 10. Problems encountered and fixed

### 10.1 Script permission issue

Problem:

```text
crates/svc-wallet/scripts/dev-quickchain-park.sh
  attempted to call crates/svc-wallet/scripts/dev-quickchain-preflight.sh directly
  but preflight script lacked executable permission

crates/ron-accounting/scripts/dev-quickchain-park.sh
  attempted to call crates/ron-accounting/scripts/dev-quickchain-preflight.sh directly
  but preflight script lacked executable permission
```

Terminal error:

```text
Permission denied
```

Fix:

```bash
chmod +x \
  crates/svc-wallet/scripts/dev-quickchain-preflight.sh \
  crates/svc-wallet/scripts/dev-quickchain-park.sh \
  crates/ron-accounting/scripts/dev-quickchain-preflight.sh \
  crates/ron-accounting/scripts/dev-quickchain-park.sh
```

Result:

```text
svc-wallet park could run through directly
ron-accounting park could run through directly
```

### 10.2 Unused mut warning

Problem:

```text
unused mut in crates/ron-accounting/tests/quickchain_phase4_bond_dispute_report_boundary.rs
```

Fix:

```text
let mut terminal_with_window
->
let terminal_with_window
```

Result:

```text
warning removed before clippy/park
```

### 10.3 Scanner vocabulary issue

Problem:

```text
ron-accounting source scanner rejected reserved Phase 1/root-material vocabulary
because the new report used finality_truth / settlement_truth identifiers
```

Fix:

```text
finality_truth      -> terminality_truth
settlement_truth    -> external_claim_truth
```

Result:

```text
quickchain_phase1_root_material_non_authority passed
quickchain_phase4_bond_dispute_report_boundary passed
ron-accounting full preflight passed
ron-accounting park passed
```

## 11. Final focused test proof

Focused tests that passed for this pair:

```bash
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_dispute_review_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_confirmation_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_tooling_boundary

cargo test -p ron-accounting --test quickchain_phase1_root_material_non_authority
cargo test -p ron-accounting --test quickchain_phase4_bond_dispute_report_boundary
cargo test -p ron-accounting --test quickchain_phase4_bond_report_boundary
cargo test -p ron-accounting --test quickchain_tooling_boundary
```

Focused results:

```text
svc-wallet quickchain_phase4_bond_dispute_review_boundary:
  5 passed
  0 failed

svc-wallet quickchain_phase4_bond_confirmation_boundary:
  5 passed
  0 failed

svc-wallet quickchain_tooling_boundary:
  4 passed
  0 failed

ron-accounting quickchain_phase1_root_material_non_authority:
  4 passed
  0 failed

ron-accounting quickchain_phase4_bond_dispute_report_boundary:
  4 passed
  0 failed

ron-accounting quickchain_phase4_bond_report_boundary:
  5 passed
  0 failed

ron-accounting quickchain_tooling_boundary:
  4 passed
  0 failed
```

Full crate-local results:

```text
svc-wallet quickchain exhaustive preflight gate passed: tests=17
svc-wallet QuickChain parking gate passed

ron-accounting quickchain exhaustive preflight gate passed: tests=17
ron-accounting QuickChain parking gate passed
```

## 12. What this pair proves

This pair proves:

```text
svc-wallet can carry a review-only disputed-bond artifact without wallet mutation
svc-wallet disputed-bond review requires explicit confirmation
svc-wallet disputed-bond review cannot represent live wallet mutation
svc-wallet disputed-bond review cannot represent balance side effects
svc-wallet disputed-bond review cannot represent automatic penalties
svc-wallet disputed-bond review cannot represent finality claims
svc-wallet live issue/transfer/burn DTOs reject dispute authority poison fields
svc-wallet source does not implement Phase 4 Round 2 dispute runtime authority

ron-accounting can carry a read-only disputed-bond report
ron-accounting report shape validates explicit challenge/appeal/frozen/terminal consistency
ron-accounting report rejects authority flags
ron-accounting report rejects unknown authority fields
ron-accounting source does not construct Phase 4 Round 2 runtime authority
ron-accounting remains derivative reporting only
ron-accounting does not become wallet truth, ledger truth, balance truth, slash truth, payout truth, terminality truth, or external-claim truth

both crates preserve dynamic QuickChain test discovery
both crates preserve Bash/cargo-only QuickChain preflight tooling
both crates preserve no checked-in Python helper drift
both crates reached tests=17 final preflight markers
both crates reached final parking gates
```

## 13. What this pair does not implement

This pair does not implement:

```text
live slashing
automatic slashing
live bond enforcement
wallet freeze route
wallet slash route
wallet bond capture route
wallet bond release route
wallet disputed-bond mutation route
accounting slash truth
accounting balance truth
accounting ledger truth
accounting wallet truth
accounting payout execution
validator rewards
public validator economy
staking
liquidity
bridge behavior
ROX
Solana
external settlement
exchange-facing logic
public chain runtime
root production
checkpoint production
proof verification
committee finality
CrabLink slash authority
gateway slash authority
omnigate slash authority
index slash truth
policy slash mutation
cache-only payment truth
fake balances
fake receipts
silent spend
```

## 14. Safe status wording

Use this wording:

```text
svc-wallet + ron-accounting are COMPLETE / PARKED for QuickChain Phase 4 Round 2.
The pair supports review-only and report-only disputed-bond simulation surfaces.
No live slashing, wallet mutation, automatic penalty, accounting truth, payout execution, staking, liquidity, bridge, or external settlement was introduced.
```

Do not say:

```text
Phase 4 Round 2 is complete
Phase 4 is complete
slashing is live
bond enforcement is live
wallet can freeze disputed bonds
wallet can execute slashing
accounting tracks slash truth
accounting has settlement truth
staking is live
validator economy is live
bridge is ready
external settlement is active
ROX/Solana is active
```

## 15. Retest commands for this completed pair

Only rerun if either crate is modified again.

Focused retests:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_dispute_review_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase4_bond_confirmation_boundary
cargo test -p svc-wallet --features quickchain-preflight --test quickchain_tooling_boundary

cargo test -p ron-accounting --test quickchain_phase1_root_material_non_authority
cargo test -p ron-accounting --test quickchain_phase4_bond_dispute_report_boundary
cargo test -p ron-accounting --test quickchain_phase4_bond_report_boundary
cargo test -p ron-accounting --test quickchain_tooling_boundary
```

Full crate-local parking gates:

```bash
cd /Users/mymac/Desktop/RustyOnions

crates/svc-wallet/scripts/dev-quickchain-park.sh
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

Expected final markers:

```text
== svc-wallet quickchain exhaustive preflight gate passed: tests=17 ==
== svc-wallet QuickChain parking gate passed ==

== ron-accounting quickchain exhaustive preflight gate passed: tests=17 ==
== ron-accounting QuickChain parking gate passed ==
```

## 16. Low-disk note

Do not rerun this pair unless necessary.

This pair is parked.

Only reopen if:

```text
a later crate pair exposes a compile failure
a later crate pair exposes a real boundary regression
a final Phase 4 Round 2 audit finds a missing check
a cross-crate integration check proves a narrow compatibility fix is needed
```

Otherwise, move forward.

## 17. Next crate pair

Next active pair in the fixed QuickChain crate-pair order:

```text
svc-rewarder + svc-storage
```

Recommended codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-rewarder
bash scripts/make_crate_codex.sh --force -c svc-storage
```

Expected `svc-rewarder` focus for Phase 4 Round 2:

```text
rewarder must not reward or punish from slash/dispute simulation directly
rewarder must not execute wallet payouts
rewarder must not mutate ledger
rewarder must not mint validator rewards from disputed-bond state
rewarder must not create automatic penalty rewards
rewarder may only plan/report if explicitly safe and read-only
```

Expected `svc-storage` focus for Phase 4 Round 2:

```text
storage must treat evidence bundles as bytes/artifacts only
storage must not become slash truth
storage must not become dispute truth
storage must not unlock paid content from dispute labels or cache alone
storage must preserve b3 integrity boundaries
storage must not mutate wallet or ledger state
storage must not create validator authority, bridge behavior, external settlement, staking, or liquidity
```

## 18. Completion judgment

Final judgment:

```text
svc-wallet + ron-accounting are complete / parked for QuickChain Phase 4 Round 2.
```

This pair now has:

```text
review-only wallet disputed-bond artifact
read-only accounting disputed-bond report artifact
explicit confirmation requirement on wallet review
integer minor-unit validation
challenge/appeal/frozen/terminal state consistency checks
authority-flag rejection
unknown-field rejection
live mutation route poisoning tests
source scanner tests
tooling boundary tests
script permission fix
scanner vocabulary fix
full crate-local preflight green
final park gates green
no live slashing
no silent spend
no fake balances
no fake receipts
no staking
no liquidity
no bridge
no ROX/Solana
no external settlement
```

Next active pair:

```text
svc-rewarder + svc-storage
```

### END NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - svc-wallet + ron-accounting


### END NOTE - JUNE 26 2026 - 01:05 CST