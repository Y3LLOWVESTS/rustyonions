---

crate: ryker
path: crates/ryker
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny supervisor for async tasks that restarts failing jobs with exponential backoff + jitter, with a temporary compatibility shim that re-exports legacy billing helpers.

## 2) Primary Responsibilities

* Provide `spawn_supervised(..)` to run an async factory with automatic restart on error (exp backoff capped at 10s + jitter).
* (Transitional) Re-export legacy billing symbols (`PriceModel`, `compute_cost`, payment validators) via an opt-in compatibility feature.

## 3) Non-Goals

* No business logic (billing, pricing) long-term — those live in `ron-billing`.
* No service orchestration, readiness, or metrics endpoints (that’s kernel/services).
* No actor framework, channels, or backpressure primitives beyond simple restart policy.

## 4) Public API Surface

* Re-exports (behind feature `billing-compat`, enabled by default right now):
  `PriceModel`, `compute_cost`, `validate_payment_block`, `validate_wallet_string` (from `ron-billing`), plus a deprecated marker module `_billing_compat_note`.
* Key functions:
  `spawn_supervised(name: &'static str, factory: impl FnMut() -> impl Future<Output = anyhow::Result<()>>) -> JoinHandle<()>`
* Traits/Types: none public besides what’s re-exported; backoff `jitter(..)` is private.
* Events / HTTP / CLI: none.

## 5) Dependencies & Coupling

* Internal crates →

  * `ron-billing` (optional via `billing-compat`): keeps old `ryker::…` billing imports working during migration. Stability: **loose** (intended to remove). Replaceable: **yes** (turn off feature and update imports).
* External crates (top) →

  * `tokio` (rt, macros, time): async runtime & timers. Mature, low risk.
  * `tracing`: structured logs. Mature, low risk.
  * `rand`: jitter generation. Mature, low risk.
  * `anyhow` (**should be added**): used in function signature; currently missing in `Cargo.toml` (see §13).
* Runtime services: none (no network/storage/crypto). Uses OS timer/PRNG via `rand`.

## 6) Config & Feature Flags

* Cargo features:

  * `billing-compat` (**default ON**): re-exports billing APIs from `ron-billing` to avoid breakage; plan to disable by default after migration and then remove.
* Env vars / config structs: none.

## 7) Observability

* Logs via `tracing`: logs start/complete/fail with error, and sleeps before restart.
* No metrics emitted (e.g., restart counters) and no readiness signal.
* Recommendation: add counters/gauges (`restarts_total{task}`, `backoff_ms{task}`, `last_error{task}` as a label/message) or expose hooks so services can increment metrics.

## 8) Concurrency Model

* Spawns a Tokio task that repeatedly calls a user-supplied async factory.
* On `Err`, sleeps with exponential backoff (start 200ms, \*2 up to 10s) plus random jitter, then restarts.
* On `Ok(())` the loop exits and the supervised task **does not** restart.
* No channels/locks here; no inbuilt cancellation/shutdown token — caller should cancel the `JoinHandle` or make the factory observe a stop signal.

## 9) Persistence & Data Model

* None. No disk or schema; no retained artifacts.

## 10) Errors & Security

* Error taxonomy: not defined here; factories return `anyhow::Result<()>`. All errors are treated as **retryable** with backoff; no circuit-breaker or max-retries.
* Security: N/A (no auth/crypto/secrets). Only touchpoint is logging — be mindful not to log secrets from factories.

## 11) Performance Notes

* Hot path is negligible; overhead is a single `tokio::spawn` and occasional logging/timer sleeps upon failure.
* Backoff caps at 10s; jitter reduces thundering-herd restarts when many tasks fail simultaneously.

## 12) Tests

* Current: **none** in the crate.
* Suggested:

  * Unit: verify backoff progression and cap; jitter bounds (e.g., within ±50%).
  * Property/fuzz: ensure no panic for arbitrary factory error types; ensure `Ok(())` halts restarts.
  * Integration: a fake factory that fails N times then succeeds; assert exactly N sleeps + restart count.

## 13) Improvement Opportunities

* **Add missing dependency:** `anyhow = { workspace = true }` in `ryker/Cargo.toml` (it appears in the public signature). Alternatively, make the signature generic: `Result<(), E> where E: std::error::Error + Send + Sync + 'static`.
* **Graceful shutdown:** accept a cancellation token or provide a `spawn_supervised_with_shutdown(stop: &CancellationToken, …)` variant; internally `select!` on stop vs. factory.
* **Observability hooks:** optional callback/trait to report `on_restart`, `on_error`, `on_success(duration)`, or expose a minimal metric counter API.
* **Circuit breaker option:** configurable max retries / cooldown to avoid infinite restarts for fatal misconfigurations.
* **Feature hygiene:** flip `billing-compat` to **off by default** once dependents migrate to `ron-billing`, then remove the re-exports in a following minor release.
* **Docs/README:** update to reflect the new focused role (supervisor), not an “experimental scratchpad”.
* **Tests:** add the set described in §12.
* **API polish:** add `spawn_supervised_named<S: Into<Cow<'static, str>>>` so names needn’t be `'static` literals.

## 14) Change Log (recent)

* 2025-09-14 — **0.2.0**: refactor to supervisor-only core; add `billing-compat` feature to re-export pricing helpers from new `ron-billing` crate; introduce jittered exp-backoff restarts.

## 15) Readiness Score (0–5 each)

* API clarity: **4** (simple, but shutdown hooks would help)
* Test coverage: **1** (none currently)
* Observability: **2** (logs only; no metrics)
* Config hygiene: **3** (clean features; missing `anyhow` dep)
* Security posture: **5** (no secrets; minimal surface)
* Performance confidence: **4** (tiny wrapper, predictable)
* Coupling (lower is better): **2** (only optional compat link to `ron-billing`)

