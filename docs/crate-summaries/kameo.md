---

crate: kameo
path: crates/kameo
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A lightweight, in-process actor toolkit (mailboxes, supervisors, and typed request/response) for building RustyOnions services with predictable concurrency, backpressure, and restarts.

## 2) Primary Responsibilities

* Provide a minimal actor runtime: `Actor` trait, `Addr<T>` handles, bounded mailboxes, and a typed `ask` pattern.
* Enforce supervision & backoff policies (crash isolation, restart limits, jitter) with metrics/log signals.
* Offer ergonomic helpers to integrate actors into our kernel (spawn, graceful shutdown, bus/health hooks).

## 3) Non-Goals

* Not a distributed actor system (no remote transport, routing tables, sharding).
* Not a replacement for the kernel bus (kameo is in-proc; bus is cross-service).
* Not a general scheduler/executor (relies on Tokio; no custom runtime).

## 4) Public API Surface

* **Re-exports:** (keep minimal) `tokio::task::JoinHandle`, `tokio::sync::{mpsc, oneshot}` when feature-gated.
* **Key types / functions / traits (expected/stable surface):**

  * `trait Actor`: `type Msg; async fn handle(&mut self, msg: Self::Msg, ctx: &mut Ctx<Self>) -> Result<()>;`
  * `struct Addr<A: Actor>`: `send(msg)`, `try_send(msg)`, `ask<R>(msg) -> Result<R>`, `close()`.
  * `struct Ctx<A: Actor>`: access to timers, spawn\_child, stop, actor name/id, supervised restarts.
  * `struct Supervisor`: strategies (`Always`, `OnPanic`, `Never`), `Backoff{min,max,jitter,reset_after}`.
  * `spawn_actor(actor, MailboxCfg) -> (Addr<A>, JoinHandle<()>)`.
  * `MailboxCfg { capacity, overflow: DropNewest|DropOldest|Reject, warn_hwm }`.
  * `AskCfg { timeout, buffer }` for typed request/response via oneshot.
* **Events / HTTP / CLI:** none; logs/metrics published for the host service to expose.

## 5) Dependencies & Coupling

* **Internal crates:**

  * *ron-kernel* (loose): optional integration to publish `KernelEvent::{ServiceCrashed,Health}` and use `Metrics` counters; replaceable: **yes**.
  * *ron-bus* (loose): not required; bus remains inter-service IPC, while kameo is in-proc.
* **External crates (top 5; likely pins/features):**

  * `tokio` (rt, sync/mpsc, time); low risk, core dependency.
  * `tracing` (spans, errors, actor ids); low risk, widely maintained.
  * `thiserror` or `anyhow` (error taxonomy); low risk.
  * `parking_lot` (fast locks for registry/counters); low risk.
  * `futures`/`async-trait` (if trait methods are async); moderate risk (dyn overhead), standard.
* **Runtime services:** none beyond process resources & clock. No network/crypto/storage in this crate.

## 6) Config & Feature Flags

* **Cargo features:**

  * `metrics` (default): register Prometheus counters/histograms in the hosting service.
  * `tracing` (default): structured spans with actor name/id and message types.
  * `bus-hooks`: emit `KernelEvent` signals via the kernel bus on crash/restart.
  * `loom-tests`: enable loom models in tests only.
* **Env vars:** none directly; consumers decide sampling/verbosity via standard `RUST_LOG` and service config.

## 7) Observability

* **Metrics (by actor label):**

  * `kameo_messages_total{actor,kind=received|handled|rejected}`
  * `kameo_mailbox_depth{actor}` (gauge) and high-watermark.
  * `kameo_handle_latency_seconds{actor}` (histogram).
  * `kameo_restarts_total{actor,reason}`; `kameo_failures_total{actor,kind=panic|error}`.
* **Health/readiness:** expose a cheap `stats()`/`is_idle()` for wiring into `/readyz` at the service layer.
* **Tracing:** span per message handle; include cause chain on failure; supervisor restart spans with backoff.

## 8) Concurrency Model

* **Execution:** one Tokio task per actor; single-threaded `handle` guarantees (no concurrent `handle` on same actor).
* **Mailboxes:** bounded `tokio::mpsc` per actor; overflow policy selected via `MailboxCfg`.
* **Backpressure:** callers use `send` (awaits when full) or `try_send` (fail fast) based on path criticality; `ask` uses bounded in-flight with timeout.
* **Supervision:** parent/child tree; on panic or error return, supervisor applies backoff (exponential + jitter) until `max_restarts` within `window` trips to `Stopped`.
* **Cancellation/Shutdown:** cooperative: `close()` stops intake; drain tail up to budget; `on_stop()` hook for cleanup.

## 9) Persistence & Data Model

* None. Kameo manages ephemeral in-memory state only (actor state + mailboxes). No DB or schema.

## 10) Errors & Security

* **Error taxonomy:**

  * `SendError::Full|Closed|Rejected` (retryable vs terminal);
  * `AskError::Timeout|MailboxClosed|Canceled` (retryable depends on caller);
  * `ActorError::Fatal|Recoverable` (guides supervisor policy).
* **Security:** not a trust boundary; no TLS/keys. Avoids `unsafe`. Guards against unbounded growth (bounded queues, HWM warnings).
* **PQ-readiness:** N/A (in-proc). Downstream services handle crypto.

## 11) Performance Notes

* **Hot paths:** mailbox enqueue/dequeue and `handle` dispatch.
* **Targets (guidance, p95 on dev iron):** enqueue < 5µs, context switch \~10–20µs, `handle` user-work dominates; end-to-end `ask` p95 < 5ms for local actors under nominal load.
* **Techniques:** avoid message serialization; prefer move semantics; batch draining (`recv_many`) when available; pre-allocate small vectors; minimize per-msg logging (sample).

## 12) Tests

* **Unit:** mailbox overflow policies; `ask` timeout; supervisor backoff windows; restart limits; `close()` drain semantics.
* **Integration:** actor trees (parent/child failure propagation), metrics labels emitted; interaction with kernel bus (feature `bus-hooks`).
* **E2E (service-level):** wire a demo service using kameo actors for request handling and assert `/readyz`/`/metrics` stability under burst.
* **Loom/proptests:** model mailbox send/close races; ensure no lost-wake deadlocks; proptest restart/backoff invariants.

## 13) Improvement Opportunities

* **Known gaps / tech debt:**

  * Clarify `Actor::handle` cancellation semantics (what happens if a long `await` is canceled during shutdown).
  * Document overflow policy guidance per call-site (when to choose `try_send` vs `send`).
  * Add `Addr::map_err` helpers to unify error mapping at service boundaries.
* **Overlap & redundancy signals:**

  * Potential duplication with `ron-kernel` “supervisor hooks” and restart logic—decide single source of truth for backoff formulas and event emission to avoid drift.
  * If any in-crate “registry” mirrors kernel registries, collapse into one.
* **Streamlining:**

  * Provide a tiny `kameo::service` adapter: actor as HTTP handler (Axum) with bounded concurrency per route.
  * Optional small LRU “inbox shadow” for prioritization (e.g., drop duplicate keys).
  * Add `recv_many`/micro-batching feature to reduce per-message overhead on hot actors.

## 14) Change Log (recent)

* 2025-09-14 — First formal crate review; aligned surface to kernel patterns; documented metrics and supervision policies.
* 2025-09-05 — Feature-gate `bus-hooks` to emit `KernelEvent` on restarts without hard coupling.
* 2025-09-01 — Bounded mailbox defaults and overflow policy introduced; ask timeout made configurable.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — surface is small and conventional (`Actor`, `Addr`, `Supervisor`), needs final trait bounds & docs.
* **Test coverage:** 2 — core paths outlined; loom/proptests planned.
* **Observability:** 3 — metrics/tracing design is clear; needs wiring + exemplars.
* **Config hygiene:** 4 — feature flags are straightforward; no env coupling.
* **Security posture:** 4 — no network/crypto; bounded resources; panic isolation.
* **Performance confidence:** 3 — bounded queues & Tokio mpsc are solid; add micro-benchmarks and `recv_many`.
* **Coupling (lower is better):** 2 — optional bus/metrics hooks only; otherwise standalone.

