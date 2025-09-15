---

crate: ron-token
path: crates/ron-token
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Domain-level facade for the token economy that currently re-exports the ledger surface while leaving room for policy, idempotency, and signing helpers.

## 2) Primary Responsibilities

* Present a **stable domain namespace** (`ron-token`) for services to depend on.
* Re-export the **ledger types/traits** so upstream code doesn’t bind to backend crate names.
* Provide a future home for **cross-cutting helpers** (idempotency, policy adapters, signing).

## 3) Non-Goals

* No storage or network I/O.
* No direct metrics or policy enforcement (that’s for services).

## 4) Public API Surface

* Re-exports:

  * `AccountId`, `Amount`, `InMemoryLedger`, `LedgerEntry`, `Op`, `Receipt`, `TokenError`, `TokenLedger` (from `ron-ledger`).
* Key types / functions / traits: currently passthrough; placeholder for future helpers.
* Events / HTTP / CLI: none.

## 5) Dependencies & Coupling

* Internal crates → `ron-ledger` (tight by intent; replaceable: **yes** via façade).
* External:

  * `serde`, `serde_json` — low risk.
* Runtime services: none.

## 6) Config & Feature Flags

* None currently; future flags could toggle helper modules (e.g., `policy`, `signing`).

## 7) Observability

* None directly; services should instrument calls.

## 8) Concurrency Model

* Same as ledger (sync API); callers decide locking strategy.

## 9) Persistence & Data Model

* Inherits model from `ron-ledger`; adds no storage.

## 10) Errors & Security

* Error taxonomy mirrors `ron-ledger`.
* Security: future **receipt signing** and **epoch roots** can land here.

## 11) Performance Notes

* Zero-overhead re-exports; future helpers should avoid allocations on hot paths.

## 12) Tests

* Thin wrapper; minimal compile-time tests; integration covered by services.

## 13) Improvement Opportunities

* Add **idempotency keys** and **per-account sequence** helpers.
* Provide **policy adapters** (wrap `TokenLedger` with `ron-policy` checks).
* Add **receipt signing** helpers (delegate to `ron-kms`).

## 14) Change Log (recent)

* 2025-09-14 — Initial façade over `ron-ledger`.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 1
* Observability: 1
* Config hygiene: 3
* Security posture: 3
* Performance confidence: 5
* Coupling (lower is better): 4

