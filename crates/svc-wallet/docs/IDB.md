
---

````markdown
---
title: svc-wallet — Invariant-Driven Blueprint (IDB)
version: 0.3.0
status: reviewed
last-updated: 2025-10-16
audience: contributors, ops, auditors
msrv: 1.80.0
pillar: 12 (Economics)
concerns: [SEC, ECON, RES, OBS, DX]
owners: [Stevan White]
---

# svc-wallet — IDB

**Crisp role:** `svc-wallet` is the end-user value plane for issuing, holding, and transferring balances while **deferring durable truth to `ron-ledger`**. It enforces runtime economic invariants (no doublespends, non-negativity, conservation, capability-bound ops, idempotency), is observable, amnesia-aware, and PQ-ready.

---

## 0) Constants & Limits (authoritative defaults)

These are policy-overridable but must always be present.

- `MAX_BODY_BYTES = 1_048_576` (1 MiB)
- `MAX_DECOMP_RATIO = 10`
- `REQ_TIMEOUT = 5s`
- `MAX_INFLIGHT = 512`
- `DEFAULT_STALENESS_WINDOW = 250ms` (read cache)
- `IDEMPOTENCY_TTL = 24h`
- `MAX_AMOUNT_PER_OP = 10^20` minor units (policy may lower)
- `MAX_DAILY_CEILING_PER_ACCOUNT = 10^22` minor units (policy)
- `MAX_ACCOUNT_TOTAL = u128::MAX - 10^9` (safety headroom)
- `NONCE_START = 1` (strictly monotonic, no reuse)

---

## 1) Invariants (MUST)

- **[I-1] No doublespends (atomic & global per account).**  
  `(account, nonce)` is reserved atomically at most once and committed at most once; replays via same `(account, nonce)` or same `Idempotency-Key` return the **identical receipt** or a **deterministic rejection**.

- **[I-2] Non-negativity (post-state).**  
  After any accepted operation, all balances `>= 0`. Overdraft attempts fail with `409/INSUFFICIENT_FUNDS`, **no state change**.

- **[I-3] Conservation (tx & batch).**  
  For any accepted transaction/batch, Σ debits == Σ credits per asset. Proof uses the **ledger batch receipt hash** returned by `ron-ledger`.

- **[I-4] Capability-only authorization.**  
  All money ops require macaroon-like capabilities scoped to `{account(s), actions, asset(s), ceilings, ttl}` validated by `ron-auth`. No ambient trust.

- **[I-5] DTO hygiene & arithmetic safety.**  
  `serde(deny_unknown_fields)`, **u128** minor units, `amount > 0`, enforce `MAX_AMOUNT_PER_OP` and daily ceilings. **No floats**.

- **[I-6] Atomicity & receipting.**  
  Success returns a short receipt `{txid, from?, to?, asset, amount, nonce, idem, ts, receipt_hash}`; failures are all-or-nothing.

- **[I-7] Ledger primacy & rebuildability.**  
  Authoritative balances live in `ron-ledger`; local cache is derivative/ephemeral/rebuildable from snapshots + deltas.

- **[I-8] Observability (golden metrics).**  
  Export: `wallet_requests_total{op}`, `wallet_rejects_total{reason}`, `wallet_idem_replays_total`, `request_latency_seconds{op}`, `wallet_inflight{op}`, `/healthz`, `/readyz`.

- **[I-9] Amnesia mode.**  
  With `amnesia=on`, no persistent writes; restart uses `ron-ledger` exclusively to recover; idempotent replays still return identical receipts from RAM-only store.

- **[I-10] PQ-ready, crypto-neutral custody.**  
  Private keys are never handled/exported by `svc-wallet`; verification delegates to `ron-auth`/`ron-kms`. PQ posture flags are honored & observable.

- **[I-11] Transport bounds & safety.**  
  `MAX_BODY_BYTES`, `MAX_DECOMP_RATIO`, `REQ_TIMEOUT`, `MAX_INFLIGHT` enforced **before** ledger IO; structured 413/429/503 with `Retry-After`.

- **[I-12] Overflow ceilings.**  
  Guard sums to prevent u128 wrap: `amount ≤ MAX_AMOUNT_PER_OP` and `account_running_total ≤ MAX_ACCOUNT_TOTAL`; excess fails with `LimitsExceeded`.

- **[I-13] No sync I/O / no direct DB.**  
  All request-path I/O is async; **no direct DB for balances**—all settlement via `ron-ledger`.

- **[I-14] Time & skew discipline.**  
  Server time source is monotonic + NTP-disciplined; idempotency TTL and nonce windows are evaluated using server time; requests may include optional `x-client-timestamp` but are never trusted alone.

### Enforced-By (traceability matrix)

| Invariant | Enforced By (module/layer) | Proved In (gate) |
|---|---|---|
| I-1 | `seq::reserve_atomic`, `idem::check_or_load` | G-1, G-9 |
| I-2 | `amounts::guard_bounds`, `balances::effective` | G-2 |
| I-3 | `ledger::commit_*` + batch `receipt_hash` verify | G-3 |
| I-4 | `auth::extract_caps` → `policy::enforce` | G-1, G-9 |
| I-5 | DTO types with `deny_unknown_fields` | G-4, G-6 |
| I-6 | transactional commit wrapper | G-1, G-3 |
| I-7 | cache invalidation on bus events | G-7 |
| I-8 | metrics layer & /healthz,/readyz | G-5, G-6 |
| I-9 | profile/amnesia guards | G-7 |
| I-10 | `ron-auth` verifier (PQ flag) | G-8 |
| I-11 | tower hardening stack | G-5, G-6 |
| I-12 | `amounts`, `policy` | G-4 |
| I-13 | lint + code review + CI grep | G-10 |
| I-14 | time source guard | G-9 |

---

## 2) Design Principles (SHOULD)

- **[P-1] Tiny surface:** `GET /v1/balance`, `POST /v1/issue`, `POST /v1/transfer`, `POST /v1/burn`, `GET /v1/tx/{txid}`.
- **[P-2] Deterministic errors:** stable codes `{BadRequest, PolicyDenied, LimitsExceeded, InsufficientFunds, NonceConflict, IdempotentReplay, UpstreamUnavailable}`.
- **[P-3] Strict separation:** policy (`ron-policy`), identity/caps (`svc-passport`/`ron-auth`), truth (`ron-ledger`), transient counters (`ron-accounting`).
- **[P-4] Backpressure-first:** shed writes on `/readyz` before exhausting upstreams; reads remain longer.
- **[P-5] Idempotency-by-default:** clients send `Idempotency-Key`; server returns prior receipt on replay (within `IDEMPOTENCY_TTL`).
- **[P-6] Audit-first:** structured events with `correlation_id`.
- **[P-7] Zero-trust retries & circuit breakers:** upstream calls wrap in timeouts, jittered exponential backoff, **tower-retry** + **tower-limit** + circuit breaker (e.g., `tower::load_shed` + custom half-open).
- **[P-8] Shardability:** partition nonce/idempotency caches by account prefix for horizontal scaling.
- **[P-9] Tight consistency:** read staleness ≤ `DEFAULT_STALENESS_WINDOW`, commit path always re-validates.

---

## 3) Implementation (HOW)

### [C-1] POST /transfer (Axum)
```rust
#[derive(Deserialize)] #[serde(deny_unknown_fields)]
pub struct TransferReq { from: Account, to: Account, asset: AssetId, amount: u128, nonce: u64 }

pub async fn post_transfer(
  State(s): State<AppState>,
  headers: HeaderMap,
  Json(req): Json<TransferReq>
) -> impl IntoResponse {
  guard_ready(&s.health)?;
  let caps = auth::extract_caps(&headers)?;
  policy::enforce(&s.policy, &caps, &req)?;
  let prior = idem::check_or_load(&s.idem, &headers)?; // returns Some(receipt) on replay
  if let Some(r) = prior { return Ok((StatusCode::OK, Json(r))); }

  seq::reserve_atomic(&s.nonces, &req.from, req.nonce)?; // no await during reserve
  amounts::guard_bounds(req.amount)?;                    // >0, ≤ MAX_AMOUNT_PER_OP

  let bal = balances::effective(&s.cache, &req.from, &req.asset).await?;
  ensure!(bal >= req.amount, Error::InsufficientFunds);

  let txid = ledger::commit_transfer(&s.ledger, &req).await?; // append-only; returns batch receipt_hash
  idem::finalize(&s.idem, &headers, txid)?;
  audit::emit("wallet.transfer.accepted", &req, txid);

  Ok((StatusCode::OK, Json(Receipt::transfer(txid, &req))))
}
````

### [C-2] GET /balance (staleness-bounded)

```rust
pub async fn get_balance(
  State(s): State<AppState>,
  Query(q): Query<BalanceQuery>
) -> impl IntoResponse {
  let snapshot = balances::read_with_staleness(&s.cache, &q.account, &q.asset, DEFAULT_STALENESS_WINDOW).await?;
  Ok((StatusCode::OK, Json(snapshot)))
}
```

### [C-3] Concurrency & atomicity

* **Nonces:** `DashMap<AccountPrefix, AtomicU64>`; `reserve_atomic` uses `fetch_update` (no lock across `.await`).
* **Idempotency:** bounded LRU + `IDEMPOTENCY_TTL`; key `(account, nonce, idem_key)` → **byte-identical** Receipt.

### [C-4] Hardening

* Tower: `Timeout(5s)`, `LoadShed`, `ConcurrencyLimit(512)`, `RateLimit`, `Decompression(10x)`, `SetRequestId`, `Trace`, **retry + circuit breaker** (half-open with backoff).

### [C-5] Error taxonomy

```rust
#[derive(thiserror::Error)]
pub enum Error {
  #[error("insufficient funds")] InsufficientFunds,
  #[error("nonce conflict")]     NonceConflict,
  #[error("replay")]             IdempotentReplay,
  #[error("policy denied")]      PolicyDenied,
  #[error("limits exceeded")]    LimitsExceeded,
  #[error("bad request")]        BadRequest,
  #[error("upstream unavailable")] UpstreamUnavailable,
}
impl IntoResponse for Error { /* stable codes & JSON bodies */ }
```

### [C-6] Metrics

Counters: `wallet_requests_total{op}`, `wallet_rejects_total{reason}`, `wallet_idem_replays_total`
Histogram: `request_latency_seconds{op}`
Gauge: `wallet_inflight{op}`

### [C-7] PQ posture

Feature `pq_hybrid` toggles verification path in `ron-auth`; posture is exposed via `/version` and metric label `pq_hybrid`.

### [C-8] Dependencies (workspace-aligned)

`axum 0.7`, `tokio 1`, `prometheus 0.14`, `serde + derive`, `dashmap`, `parking_lot`, `thiserror`, `tokio-rustls 0.26`, `rand 0.9`, `tower` (+ retry/load-shed).
Internal: `ron-ledger-client`, `ron-auth`, `ron-policy`, `ron-proto`, `ron-accounting`.

---

## 4) Acceptance Gates (PROOF)

**All gates have tooling + numeric pass criteria. CI fails hard on violation.**

* **[G-1] Property tests — doublespend/idempotency**
  *Tool:* `proptest`
  *Pass:* 10k cases; 0 violations; replays return identical receipts; `wallet_idem_replays_total` increments.

* **[G-2] Non-negativity**
  *Tool:* integration
  *Pass:* Across 1k randomized accounts, all overdrafts reject with `409`; balances unchanged.

* **[G-3] Conservation via ledger receipt**
  *Tool:* ledger harness
  *Pass:* 0 Σ mismatch across 5k tx; batch `receipt_hash` verifies.

* **[G-4] Bounds & overflow**
  *Tool:* fuzz (`cargo-fuzz`), unit
  *Pass:* 0 panics/UB; `LimitsExceeded` for >`MAX_AMOUNT_PER_OP` or total overflow.

* **[G-5] Readiness & shedding**
  *Tool:* chaos
  *Pass:* Induced ledger stall (2s) flips `/readyz` <200ms; POST=503 with `Retry-After`; GET `/balance` stays 200.

* **[G-6] Metrics presence & movement**
  *Tool:* e2e scrape
  *Pass:* All golden metrics present; buckets move under 1k rps; p95 <120ms lab target.

* **[G-7] Amnesia matrix**
  *Tool:* micronode profile
  *Pass:* No files written; restart restores solely from ledger; replays return identical receipt.

* **[G-8] PQ posture matrix**
  *Tool:* feature build
  *Pass:* `pq_hybrid` on/off both verify caps correctly.

* **[G-9] Load & replay soak**
  *Tool:* k6/vegeta
  *Pass:* 10k rps for 5 min, 1% duplicate idem; p99 <250ms; error <0.3% (retryable only); 0 invariant breaches.

* **[G-10] Supply-chain & safety**
  *Tool:* `cargo audit`, `cargo-deny`, `clippy -D warnings`, `miri` core paths
  *Pass:* all green; no UB; no banned deps.

* **[G-11] Mutation testing**
  *Tool:* `cargo-mutants` (core modules)
  *Pass:* ≥85% mutants killed on seq/idemp/balances modules.

---

## 5) Anti-Scope (Forbidden)

* No **authoritative** balances or direct DB writes in `svc-wallet`.
* No **ambient auth** (cookies/IP lists) for money ops.
* No **floats**; no implicit currency conversion.
* No **unbounded** queues; no locks held across `.await`.
* No **persistence** in Micronode amnesia mode.
* No **sync I/O** in request path; no **custom crypto** (delegate to `ron-kms`).
* No cross-asset atomic swaps (lives in higher-level service).

---

## 6) References

* Full Project Blueprint; Hardening Blueprint v2.0; Scaling Blueprint v1.3.1
* Concurrency & Aliasing Blueprint v1.3
* 12 Pillars (Pillar 12 — Economics)
* Six Concerns (SEC, ECON, RES, OBS, DX)
* `ron-ledger`, `ron-accounting`, `ron-auth`, `ron-policy`, `ron-kms` docs

```
