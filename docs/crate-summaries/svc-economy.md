
---

crate: svc-economy
path: crates/svc-economy
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

HTTP service exposing a policy-ready API for mint/burn/transfer/balance/supply over the ledger with golden metrics and health/readiness endpoints.

## 2) Primary Responsibilities

* Provide a minimal **REST API** over the token ledger (`/mint`, `/burn`, `/transfer`, `/balance/:acct`, `/supply`).
* Expose **ops surfaces**: `/metrics` (Prometheus), `/healthz`, `/readyz`, `/version`.
* Maintain **service-local concurrency safety** via `RwLock` around the ledger.

## 3) Non-Goals

* Not a billing engine or pricing calculator (that’s `ron-billing`).
* No external blockchain integration.
* No persistence (in-memory only) in this draft.

## 4) Public API Surface

* Re-exports: none.
* HTTP endpoints:

  * `POST /mint` `{account, amount, reason?}` → `{receipt}`
  * `POST /burn` `{account, amount, reason?}` → `{receipt}`
  * `POST /transfer` `{from, to, amount, reason?}` → `{receipt}`
  * `GET /balance/:account` → `{account, balance}`
  * `GET /supply` → `{total_supply}`
  * Ops: `GET /`, `/version`, `/healthz`, `/readyz`, `/metrics`
* CLI: none (env-driven).

## 5) Dependencies & Coupling

* Internal crates → `ron-ledger` (ledger operations; tight, replaceable: **yes** by trait), `ron-token` optional later.
* External (top 5):

  * `axum 0.7` (HTTP), `tokio 1.x` (runtime), `prometheus 0.14` (metrics),
  * `tracing`/`tracing-subscriber` (logs), `parking_lot` (locks).
* Runtime services: Network (TCP listener). No DB/Crypto yet.

## 6) Config & Feature Flags

* Env vars:

  * `ECONOMY_ADDR` (default `127.0.0.1:3003`).
* Feature flags: none yet; later: `policy`, `audit`, `tls`.

## 7) Observability

* Metrics:

  * `tx_total{op}` (success counts), `tx_failed_total{op,reason}`, `request_latency_seconds` (histogram).
* Health:

  * `GET /healthz` liveness; `GET /readyz` gated by `AtomicBool`.
* Logs:

  * `tracing` with `RUST_LOG` support; uptime in `/`.

## 8) Concurrency Model

* Single axum server task; graceful shutdown on `CTRL-C`.
* Ledger protected by **`RwLock<InMemoryLedger>`** (write for tx ops, read for `balance/supply`).
* Backpressure: Hyper defaults only; timeouts/rate-limits not yet installed.

## 9) Persistence & Data Model

* In-memory ledger (non-durable). Entries kept in a vector; no eviction.
* Future: replace `InMemoryLedger` with a `TokenLedger` backend backed by SQLite/sled; add **checkpoints** and recovery.

## 10) Errors & Security

* Error taxonomy surfaced as HTTP:

  * `400` → `zero_amount`, `insufficient_funds`
  * `500` → `overflow`, `internal`
* Security:

  * No auth/z; no TLS/mTLS; **future**: wrap handlers with `ron-policy` and `ron-auth` envelopes; terminate TLS via `tokio-rustls`.
* PQ-readiness: N/A now; plan to integrate PQ-safe signature paths via `ron-kms`.

## 11) Performance Notes

* Hot paths: `mint/burn/transfer` (write lock); `balance/supply` (read lock).
* Targets (draft): p95 < 5–10 ms locally; throughput gated by single write lock (good enough for MVP).

## 12) Tests

* Unit: handler happy-paths and error classifications.
* Integration: bind ephemeral port; exercise all endpoints; scrape `/metrics`.
* Property: **supply conservation** across random sequences of ops.
* (Future) Concurrency: race tests on `transfer` contention.

## 13) Improvement Opportunities

* **Policy**: require roles for `/mint` & `/burn` (integrate `ron-policy`).
* **Audit**: emit epoch roots/receipts to `ron-audit`.
* **Persistence**: adopt `ron-ledger-sqlite` (transactions, idempotency keys).
* **Ops**: add tower middlewares (`Timeout`, `RateLimit`, `ConcurrencyLimit`, compression).
* **Security**: enable TLS (tokio-rustls); add request signing via `ron-auth`.

## 14) Change Log (recent)

* 2025-09-14 — Initial service with REST API, metrics, and health/readiness.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 2
* Observability: 4
* Config hygiene: 3
* Security posture: 2
* Performance confidence: 4
* Coupling (lower is better): 4

---
