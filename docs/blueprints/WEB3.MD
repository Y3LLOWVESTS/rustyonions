````md
---
title: WEB3 — ROC Internal Economy Blueprint (RustyOnions)
status: active
last-updated: 2026-04-16
msrv: 1.80.0
owners: [Stevan White]
profile: WEB3
---

# WEB3 — ROC Internal Economy Blueprint

This document is the **single north star** for the RustyOnions “web3/value-plane” work. It defines the minimal, correct, deterministic path to ship **ROC** (internal token/accounting) as a functioning closed-loop economy inside RustyOnions. **ROX/external chain settlement is explicitly deferred** until the final phase; we only preserve clean seams for it.

---

## 0) One-sentence mission

Ship **ROC** as an internal, deterministic economy (ledger + wallet + accounting + rewarder + enforcement + visibility) with **at least one real in-network use case** (recommended: **pay-for-storage/pinning**) and provable invariants (no doublespends, conservation, non-negativity, idempotency, auditability).

---

## 1) Scope

### 1.1 In scope now (ROC: internal value plane)
- A single internal unit of account: **ROC**
- Durable ledger (`ron-ledger`): append-only truth + replay + snapshots
- Wallet service (`svc-wallet`): API boundary for balances, transfers, mint/issue, burn, receipts, nonces, idempotency
- Accounting (`ron-accounting`): metering + pricing inputs (not truth)
- Rewarder (`svc-rewarder`): deterministic issuance from accounting/policy signals
- Enforcement integration: at least one live “pay-for-X” path (first: **storage/pinning**)
- Observability: `/metrics`, `/healthz`, `/readyz` + ROC-specific golden metrics
- Security: capability/policy gating, deterministic error taxonomy, amnesia-mode rules

### 1.2 Explicitly deferred (NOT now)
- **ROX / Solana / external settlement**: on-chain mint/burn, bridge contracts, proof/attestations to public chain
- Staking, governance voting, public exchanges, liquidity pools
- ZK/advanced privacy (can be a later phase)
- Ads economy and interop bridges (unless specifically required to prove ROC loop)

### 1.3 Definition of “done” for WEB3 v1
ROC is “shipped” when:
1) The ledger is durable and replayable (crash-safe, deterministic),
2) Wallet is the sole mutation front-door with nonce+idempotency (no doublespends),
3) At least one real in-network use case charges ROC (paid storage/pinning recommended),
4) Rewarder issues ROC deterministically from measurable contributions,
5) Metrics + dashboard (or admin endpoints) show the economy moving.

---

## 2) Design principles and non-negotiable invariants

### 2.1 Determinism
- Same inputs must yield the same outputs across restarts:
  - ledger replay
  - receipts
  - reward epochs
  - enforcement decisions
- No floating point in accounting/ledger math. Use integer minor units.

### 2.2 Ledger truth vs counters
- **`ron-accounting` is transient counters**
- **`ron-ledger` is append-only truth**
- Rewarder consumes counters but commits truth via wallet → ledger.

### 2.3 Conservation (transaction-level)
For transfer-like transactions:
- Sum(credits) == Sum(debits)
- The only exceptions are:
  - **mint/issue** (creates supply)
  - **burn** (destroys supply)
Both must be explicitly authorized and auditable.

### 2.4 Non-negativity (account-level)
- By default: **no account may go negative**.
- If overdraft/credit is ever added later, it must be explicit per-account policy with separate invariants.

### 2.5 No doublespends
- Every account mutation must be protected by:
  - per-account sequence / nonce
  - idempotency keys (retry-safe)
  - deterministic conflict handling

### 2.6 Capability/policy gating
- Wallet mutation endpoints require explicit capability scopes.
- Mint/burn require stricter scopes than transfer.
- Enforcement decisions (e.g., pay-for-storage) must be policy-aware.

### 2.7 “Hold → Capture → Release” for paid operations
Any pay-for-X flow must use:
1) **Hold** maximum estimated amount (escrow)
2) **Capture** actual cost on completion
3) **Release** remainder back to payer
Never “best-effort” charge without reservation.

### 2.8 Observability is mandatory
Every ROC service exposes:
- `/metrics`, `/healthz`, `/readyz` (truthful)
- Golden metrics (see §10)

### 2.9 Amnesia mode must be respected
When the node profile is “amnesia”:
- ROC must never silently persist data to disk.
- Persistence must be explicit and configurable.

---

## 3) System architecture (high-level)

### 3.1 Planes
- **Core plane**: networking/overlay/storage/index/gateway/identity/ops (already exists)
- **Value plane (ROC)**: ledger/wallet/accounting/rewarder/enforcement

Core plane produces *metering signals* and requests that require *economic authorization*. Value plane authorizes and settles.

### 3.2 Primary services and their responsibilities
- `ron-ledger`:
  - append-only entries/transactions
  - replay to rebuild state
  - snapshots/checkpoints
  - receipts
- `svc-wallet`:
  - validates requests, caps/policy, nonce/idempotency
  - calls ledger engine to commit
  - serves balances/receipts
- `ron-accounting`:
  - collects metering events (bytes stored, bytes served, pins, sessions, uptime)
  - produces compact accounting snapshots for rewarder and pricing
- `svc-rewarder`:
  - deterministic reward algorithm
  - produces issuance intents
  - commits via wallet/ledger
- Enforcement (first integration):
  - gateway/storage path requires wallet authorization for paid operations

### 3.3 Diagram
```mermaid
flowchart LR
  subgraph Core["Core Plane"]
    GW[svc-gateway]
    ST[svc-storage]
    IX[svc-index]
    ED[svc-edge]
  end

  subgraph Value["Value Plane (ROC)"]
    AC[ron-accounting]
    RW[svc-rewarder]
    WL[svc-wallet]
    LG[ron-ledger]
  end

  GW -->|paid write request| WL
  WL -->|commit tx| LG

  ST -->|metering| AC
  GW -->|metering| AC
  ED -->|metering| AC

  AC -->|epoch snapshot| RW
  RW -->|issuance intents| WL
````

---

## 4) Data model (ROC)

### 4.1 Units

* Use **minor units** (integer). Example: 1 ROC = 1_000_000 minor units (or 1_000, or 1 — choose once and lock it).
* Amount type:

  * Ledger internal math: `u128` (safer for aggregation)
  * DTO fields: `u64` unless proven insufficient
* Overflow must be checked; never wrap.

### 4.2 Accounts

Accounts are strings but must be canonical:

* Recommended canonical format:

  * `t:<tenant>/u:<user>/w` (user wallet)
  * `t:<tenant>/s:<service>/<bucket>` (service buckets)
  * `t:<tenant>/treasury`
  * `t:<tenant>/escrow/<hold_id>`
* Wallet must validate account strings for:

  * length bounds
  * allowed charset
  * prefix semantics (optional but recommended)

### 4.3 Transaction and entry shapes

A transaction is an atomic set of entries:

* Each entry is a debit or credit record:

  * `account`
  * `amount_minor`
  * `sign` (+1 credit, -1 debit)
  * optional memo
* A transaction has:

  * `txid` (unique, stable)
  * `idempotency_key`
  * `created_at`
  * `kind` (transfer/mint/burn/hold/capture/release/fee/reward/etc.)
  * `entries: Vec<Entry>`
  * `policy_context` (optional, for audits)

### 4.4 Receipts

Every committed transaction yields a deterministic receipt:

* Fields:

  * `txid`
  * `status` (committed/duplicate/rejected)
  * `reason` (stable taxonomy for rejects)
  * `balance_deltas` (optional)
  * `receipt_hash` (BLAKE3 over canonical receipt form)
* Receipt hash is used for:

  * dedupe
  * audit chain
  * dashboards

### 4.5 Holds (escrow)

A hold is a special transaction sequence:

* **hold**:

  * debit payer wallet (or move into escrow) up to max estimate
* **capture**:

  * move from escrow to payee/treasury actual cost
* **release**:

  * move remainder from escrow back to payer
    Holds must be idempotent and must have a terminal state:
* captured fully
* released fully
* partially captured then released remainder

---

## 5) `ron-ledger` blueprint (truth engine)

### 5.1 Ledger responsibilities

* Append-only storage of committed transactions (or entries grouped into transactions)
* Deterministic replay to rebuild:

  * balances
  * holds state
  * supply totals
* Snapshots/checkpoints to speed replay
* Deterministic reject taxonomy (insufficient funds, bad nonce, etc.)
* Query APIs for:

  * account balance
  * transaction receipt
  * transaction history (paged)

### 5.2 Storage backends

WEB3 v1 must support at least:

* **RAM backend** (tests, amnesia demo)
* **Durable backend** (for non-amnesia profiles)
  Options:
* sled
* sqlite
* rocksdb
  Pick what the workspace already standardizes on. The key is replay correctness and atomicity.

### 5.3 Append-only log model

* A transaction commit is appended as one record:

  * `txid`, metadata, entries
* Commit must be atomic:

  * either the full record is present, or not present
* Crash recovery:

  * on startup, scan log, rebuild state, validate checkpoints

### 5.4 Indexes and snapshots

To serve reads efficiently:

* Maintain a balance index keyed by account
* Periodic snapshot records:

  * snapshot height
  * account balances (compressed)
* Replay strategy:

  * load latest snapshot
  * apply log records after snapshot

### 5.5 Concurrency model

* Single writer task for ledger commits (via bounded channel)
* Reads can be concurrent (snapshot + index)
* Never hold locks across `.await` in commit path

### 5.6 Deterministic validation

Validation order must be deterministic:

1. Validate DTO bounds (sizes, counts, charset)
2. Validate tx kind rules (mint requires authority, etc.)
3. Validate conservation rules if applicable
4. Validate nonce/idempotency
5. Validate non-negativity constraints
6. Commit append-only record
7. Update indexes
8. Emit receipt

### 5.7 Public API (crate-level)

Expose a narrow trait that wallet uses:

* `submit(tx) -> Receipt`
* `balance(account) -> Amount`
* `get_receipt(txid) -> Option<Receipt>`
* `list(account, cursor) -> Page<TxSummary>`
* `health_snapshot()`

---

## 6) `svc-wallet` blueprint (network boundary)

### 6.1 Wallet responsibilities

* Authenticate and authorize (capability/policy)
* Enforce nonce and idempotency
* Construct ledger transactions for:

  * issue/mint
  * transfer
  * burn
  * holds/capture/release
* Expose balance and receipt queries
* Provide `/metrics`, `/healthz`, `/readyz`

### 6.2 Minimal HTTP API (v1)

All endpoints are JSON; sizes bounded by 1 MiB request body maximum.

#### `GET /v1/balance?account=...`

* Response: `{ account, balance_minor, as_of_height }`

#### `POST /v1/transfer`

* Request: `{ from, to, amount_minor, nonce, idempotency_key, memo? }`
* Response: `Receipt`

#### `POST /v1/issue` (privileged)

* Request: `{ to, amount_minor, idempotency_key, memo? }`
* Response: `Receipt`

#### `POST /v1/burn` (privileged or user-allowed if policy says so)

* Request: `{ from, amount_minor, nonce, idempotency_key, memo? }`
* Response: `Receipt`

#### `POST /v1/hold`

* Request: `{ from, amount_minor_max, nonce, idempotency_key, context }`
* Response: `{ hold_id, receipt }`

#### `POST /v1/capture`

* Request: `{ hold_id, amount_minor_actual, payee, idempotency_key }`
* Response: `Receipt`

#### `POST /v1/release`

* Request: `{ hold_id, idempotency_key }`
* Response: `Receipt`

#### `GET /v1/tx/{txid}`

* Response: `Receipt` or 404

### 6.3 Headers and idempotency

* Support `X-Request-Id` (propagate into logs/metrics)
* Support either:

  * `X-Idempotency-Key`, or
  * idempotency key inside request JSON
    Pick one and standardize.

Idempotency rules:

* Same key + same canonical request = return prior receipt (200)
* Same key + different canonical request = reject (409 conflict)

### 6.4 Nonce rules

* Per-account nonce increments on each successful mutation.
* Nonce can be:

  * strict next value (`expected_nonce`)
  * or monotonic with window (not recommended for v1)
    Keep it strict for v1: easiest to prove.

### 6.5 Error taxonomy mapping

Wallet must map ledger rejects to stable HTTP statuses:

* 400 bad request (malformed, invalid)
* 401/403 unauthorized/forbidden (missing cap, policy deny)
* 404 not found (missing tx)
* 409 conflict (idempotency mismatch, nonce conflict)
* 413 too large (body limits)
* 429 rate limited
* 503 unavailable (upstream stall, readiness shed)

### 6.6 Readiness and shedding

Wallet must degrade first:

* When ledger stalls / DB is slow / disk headroom low:

  * `/readyz` flips false
  * write endpoints shed or 503 quickly
* Never allow “slow death” where it accepts writes but can’t commit.

---

## 7) `ron-accounting` blueprint (metering, not truth)

### 7.1 Accounting responsibilities

* Emit metering events such as:

  * bytes stored
  * bytes served (egress)
  * pins created / pin-seconds
  * successful requests
  * provider uptime contributions (optional)
* Provide compact snapshots to rewarder:

  * per tenant, per provider, per epoch

### 7.2 Event schema

Define a minimal schema:

* `timestamp`
* `tenant`
* `subject` (provider/user/service)
* `metric_kind` (bytes_stored, bytes_served, pin_seconds, req_ok, etc.)
* `value` (u64)

Accounting is *allowed* to be lossy within bounded error for v1 (e.g., sampling), but it must be deterministic given the same inputs.

---

## 8) Pricing model (pay-for-X) and the first use case

### 8.1 Pick one use case to ship first

Recommended: **pay-for-storage/pinning** (paid writes)

Rationale:

* Simple to reason about
* Forces end-to-end integration (gateway/storage ↔ wallet ↔ ledger)
* Visible to users

### 8.2 Pricing inputs

Define a price table (configurable):

* `price_put_per_byte_minor`
* `price_pin_per_second_minor`
* `min_charge_minor`
* optional: `price_get_per_byte_minor` (defer egress charging until after storage works)

### 8.3 Enforcement location

First enforcement should occur at ingress/admission (gateway) before heavy work:

* For paid write routes:

  * estimate cost
  * call wallet hold (or direct debit for simple v0)
  * only proceed if success

### 8.4 Hold-based charging flow (recommended)

1. Gateway computes `max_estimate`
2. `POST /v1/hold` from user wallet for max
3. Perform storage/pin operation
4. Compute actual cost
5. `POST /v1/capture` to treasury/provider for actual
6. `POST /v1/release` remainder to user

If the operation fails:

* release hold (unless policy says keep fee for attempt)

---

## 9) `svc-rewarder` blueprint (issuance engine)

### 9.1 Rewarder responsibilities

* Run on epochs (e.g., every N minutes)
* Consume accounting snapshot + policy
* Compute deterministic issuance plan:

  * who gets how much ROC
  * why (category)
* Commit issuance via wallet/ledger:

  * `issue` transactions (privileged)

### 9.2 Reward formula (v1 simple)

Start with deterministic, explainable rules:

* Reward providers for:

  * pin-seconds served
  * successful bytes served
* Optional: reward storage contributions (bytes stored) with decay
* Anti-gaming:

  * cap per epoch
  * minimum thresholds
  * policy filters

### 9.3 Idempotency and replay safety

Rewarder must use:

* epoch id
* deterministic issuance idempotency keys
  So re-running the same epoch cannot double mint.

### 9.4 Audit outputs

Rewarder should emit:

* summary receipts
* per-epoch report (JSON) stored as content-addressed artifact (optional, nice)

---

## 10) Observability (golden metrics)

### 10.1 Required endpoints

Every ROC service:

* `/metrics`
* `/healthz`
* `/readyz`

### 10.2 Metrics (minimum set)

Ledger:

* `roc_ledger_commits_total{result=ok|reject}`
* `roc_ledger_replay_seconds` histogram
* `roc_ledger_checkpoint_height` gauge
* `roc_ledger_balance_reads_total`

Wallet:

* `roc_wallet_requests_total{route, result, reason}`
* `roc_wallet_latency_seconds{route}` histogram
* `roc_wallet_shed_total`
* `roc_wallet_idempotency_hits_total`
* `roc_wallet_nonce_conflicts_total`

Rewarder:

* `roc_rewarder_epochs_total{result}`
* `roc_rewarder_minted_minor_total`
* `roc_rewarder_compute_seconds` histogram

Enforcement:

* `roc_payments_authorizations_total{result}`
* `roc_payments_charged_minor_total`
* `roc_payments_holds_outstanding` gauge

### 10.3 Logging rules

* Never log secrets or full capability tokens
* Log txids, receipt hashes, reason codes, request ids
* In amnesia mode: suppress disk logs and keep retention minimal

---

## 11) Security model

### 11.1 Threats addressed in v1

* Double spend via retries
* Replay attacks
* Unauthorized mint/burn
* DoS via huge requests
* Economic bypass (skip payment path)

### 11.2 Controls

* Capability gating on wallet endpoints
* Strict size/time limits
* Idempotency keys
* Per-account nonce
* Deterministic reject taxonomy for consistent client behavior
* Audit trail integration (receipt hashes)

### 11.3 Amnesia-mode constraints

Define explicit behavior:

* If amnesia mode is ON:

  * ledger defaults to RAM backend
  * no persistence unless explicitly configured
* If persistence is required:

  * amnesia must be OFF or explicitly overridden (documented)

---

## 12) Testing and verification (must bite)

### 12.1 Unit tests (required)

* ledger conservation
* ledger non-negativity
* idempotency behavior
* nonce conflict behavior
* receipt hash determinism

### 12.2 Property tests (required)

* For randomized transaction sequences:

  * replay yields identical balances/receipts
  * no negative balances if policy forbids
  * conservation holds for transfer txs

### 12.3 Fuzzing (recommended early)

* receipt parsing
* tx decoding
* wallet request parsing (size/ratio guards)

### 12.4 Chaos tests (required before “ship”)

* kill wallet mid-commit
* kill ledger mid-checkpoint
* restart during holds
* retry storms with same idempotency keys

### 12.5 Perf sanity

* ledger commit p95 under target threshold on dev machine
* wallet latency p95 under target threshold
* replay bounded by snapshots

---

## 13) Milestones (execution plan)

### M1 — Ledger becomes real

Deliverables:

* append-only record format
* replay
* balance index
* receipts
* tests: conservation + non-negativity + replay determinism

Exit gate:

* `cargo test -p ron-ledger` passes with meaningful tests

### M2 — Wallet becomes real

Deliverables:

* `/v1/balance`, `/v1/transfer`, `/v1/issue`, `/v1/burn`, `/v1/tx/{txid}`
* nonce + idempotency
* cap/policy gating for privileged ops
* `/metrics /healthz /readyz`

Exit gate:

* wallet can run locally and handle mint → transfer → query receipt

### M3 — First paid use case (pay-for-storage/pinning)

Deliverables:

* pricing table
* enforcement at gateway/admission
* hold/capture/release implemented (or direct debit for a tiny v0, then migrate to holds)

Exit gate:

* paid write fails with 0 ROC, succeeds after mint, and charges ROC correctly

### M4 — Rewarder loop

Deliverables:

* deterministic epoch compute
* idempotent issuance
* mint receipts
* basic anti-gaming caps

Exit gate:

* rewarder run increases provider balances deterministically and is replay-safe

### M5 — Visibility

Deliverables:

* dashboard/admin endpoints show supply, treasury, balances, recent receipts
* metrics demonstrate economy movement

Exit gate:

* a “WOW demo” exists: mint → spend → reward → observe

### M6 — ROX seam (still deferred)

Deliverables:

* clean interface boundary for future bridge
* no chain deps, no settlement logic

Exit gate:

* documented seam only (no Solana code)

---

## 14) Developer workflow and repo discipline

### 14.1 Codebundle + review artifact

* Keep a Rust-only codebundle script in `scripts/` and treat it as part of governance.
* Ensure it excludes build outputs and irrelevant directories.

### 14.2 “Paste-ready” rule for fixes

When changes are needed due to compile errors or regressions:

* provide full file replacements with exact paths
* include Cargo.toml if modified
* terminal commands must omit leading `#`

### 14.3 Minimal smoke demo commands (example)

(Replace service names/ports with your actual run configs.)

Build:
cargo build -p ron-ledger -p svc-wallet -p svc-rewarder

Run wallet:
cargo run -p svc-wallet

Mint/dev faucet call:
curl -sS -X POST [http://127.0.0.1:PORT/v1/issue](http://127.0.0.1:PORT/v1/issue) -d '{"to":"t:demo/u:alice/w","amount_minor":1000000,"idempotency_key":"demo-issue-1"}' -H 'content-type: application/json'

---

## 15) Appendix — canonical reject reasons (stable strings)

Define stable, immutable reason strings for metrics and client logic:

* `bad_request`
* `unauthorized`
* `forbidden`
* `not_found`
* `conflict_nonce`
* `conflict_idempotency`
* `insufficient_funds`
* `conservation_violation`
* `amount_overflow`
* `too_large`
* `rate_limited`
* `unavailable`
* `internal`

These reason codes must remain stable once published.

---

# End of WEB3.MD

```
```
