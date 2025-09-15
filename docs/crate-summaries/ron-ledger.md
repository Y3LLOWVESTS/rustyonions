
---

crate: ron-ledger
path: crates/ron-ledger
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Storage-agnostic token ledger core providing a trait, typed entries/receipts, and a reference in-memory implementation with strict supply/balance invariants.

## 2) Primary Responsibilities

* Define the canonical **ledger trait** (`TokenLedger`) and domain types (accounts, entries, receipts, errors).
* Provide a **reference in-memory implementation** (`InMemoryLedger`) for rapid integration/testing.
* Enforce **safety invariants**: non-negative balances, supply conservation, overflow checks.

## 3) Non-Goals

* No persistence backends (SQLite/sled/rocks) in this crate.
* No auth/policy, billing, or network endpoints.
* No async I/O; stays sync to keep backends flexible.

## 4) Public API Surface

* Re-exports: N/A (this is the source of truth).
* Key types / functions / traits:

  * `trait TokenLedger { mint/burn/transfer/balance/total_supply/entries }`
  * `struct InMemoryLedger`
  * `struct AccountId(String)`, `type Amount = u128`
  * `enum Op { Mint, Burn, Transfer }`
  * `struct LedgerEntry { id, ts_ms, op, from, to, amount, reason, supply_after }`
  * `struct Receipt { entry_id, balance_after, supply_after }`
  * `enum TokenError { ZeroAmount, InsufficientFunds{…}, Overflow }`
* Events / HTTP / CLI: none.

## 5) Dependencies & Coupling

* Internal crates → none (pure domain lib) \[replaceable: yes].
* External crates (top 5):

  * `serde`, `serde_json` (DTOs) — low risk, permissive.
* Runtime services: none (no Network/Storage/OS/Crypto coupling).

## 6) Config & Feature Flags

* None currently; future features may add backends behind flags.

## 7) Observability

* None baked in (no metrics/logs); callers/services should emit metrics.

## 8) Concurrency Model

* Single-threaded data structure; callers wrap with a lock (e.g., `parking_lot::RwLock`).
* No internal channels/timeouts; retries handled by callers.

## 9) Persistence & Data Model

* In-memory: `HashMap<AccountId, Amount>` + `Vec<LedgerEntry>`.
* `id: u64` monotonic (wraps on overflow), `ts_ms: u128` wall-clock.
* No retention/compaction; callers responsible if needed.

## 10) Errors & Security

* Error taxonomy: **terminal** (`ZeroAmount`, `Overflow`), **retryable-by-user** (`InsufficientFunds` when state changes).
* Security: no auth/z; no secrets/TLS; PQ-readiness N/A.

## 11) Performance Notes

* Hot paths: `mint/burn/transfer/balance` are O(1) hash ops; entries push O(1).
* Expected latency: sub-µs on modern CPUs for single-threaded ops.

## 12) Tests

* Unit: invariants (supply conservation; no negative balances; overflow guard).
* Property: commutativity where applicable (non-commutative ops explicitly tested).
* Concurrency: external (service-level) with locking.

## 13) Improvement Opportunities

* Add **iterator/streaming** over entries; paged scans.
* Introduce **idempotency** helpers (sequence numbers per account).
* Provide **backends** in sibling crates: `ron-ledger-sqlite`, `ron-ledger-sled`.
* Optional **audit hooks** (callback on entry append).

## 14) Change Log (recent)

* 2025-09-14 — Initial release with `TokenLedger` and `InMemoryLedger`.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 2
* Observability: 1
* Config hygiene: 3
* Security posture: 3
* Performance confidence: 5
* Coupling (lower is better): 5

---
