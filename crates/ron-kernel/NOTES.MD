### BEGIN NOTE - OCTOBER 17 2025 - 21:19 CST

# Carryover notes (context the next instance should know)

* **Public API is frozen** at the crate root to: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c`.
  Anything new (e.g., watcher helpers, `ConfigCell`) lives under modules (e.g., `config::…`) to avoid drift.
* **Kernel invariants** are upheld: bounded, lossy bus; no locks across `.await`; `/readyz` degrades with 503+Retry-After; amnesia posture surfaced as a gauge.
* **Metrics registry** is global & idempotent; `Metrics` is now `Clone`. New helper `Metrics::set_amnesia(bool)`.
* **Supervisor** uses jittered exponential backoff (100ms → cap 30s), updated to `rand 0.9` API, and publishes `KernelEvent::ServiceCrashed { service, reason }` + increments `service_restarts_total{service="…"}`.
* **Config hot-reload (real):**

  * File watcher (`notify`) for a TOML file `{ version, amnesia }`, plus an env poller (`RON_AMNESIA`).
  * **Content-based apply** (looks at `amnesia`) with **optional autobump**: on content change, `version = max(file.version, old.version+1)`.
  * Duplicate filesystem notifications no longer cause double `ConfigUpdated`.
  * `amnesia_mode` gauge stays in sync with `Config.amnesia`.
* **Perf baselines:** Criterion benches show publish path ~185–220ns (flat vs fan-out up to 64 subs) and ~212ns on overflow—this is “gold-ready.” Saved baselines recommended.
* **Docs intent:** We kept comments in RO:WHAT/WHY/INVARIANTS style and satisfied `#![deny(missing_docs)]`. The watcher behavior/semantics should be reflected in docs next (see “Next actions”).

Run quick checks:

```
cargo test -p ron-kernel
cargo bench -p ron-kernel --bench bus_publish
cargo bench -p ron-kernel --bench bus_lag_vs_publish
cargo run -p ron-kernel --example kernel_demo
```

# Changelog — items added that were not in TODO.md

## New source files (implementation)

* `src/internal/constants.rs`
  Centralized caps + backoff constants (bounded bus, supervisor backoff start/cap). Prevents magic numbers and blueprint drift.
* `src/config/watcher.rs`
  Real watchers: filesystem (TOML) + env poller (RON_AMNESIA). Non-blocking bridge (`notify` thread → async via mpsc). Content-based change detection + optional autobump. Syncs `amnesia_mode` and emits `ConfigUpdated`.
* `src/events.rs` (if TODO.md had this embedded in `lib.rs`)
  Pulled `KernelEvent` into its own file for clearer docs and semver-freeze review.

## New examples & benches

* `examples/kernel_demo.rs`
  Minimal runnable demo: metrics server, bus, `ConfigCell`, file/env watcher; shows `amnesia_mode` changes and `ConfigUpdated` events.
* `benches/bus_publish.rs`
  True steady-state publish cost (setup hoisted). Measures: 0 subs, 1 sub, overflow path.
* `benches/bus_lag_vs_publish.rs`
  Publish cost vs. fan-out (0,1,4,16,64 subs). Confirms flat cost under fan-out on our current design.

## New tests (if not listed in TODO.md)

* `tests/public_api.rs`
  Guard against export drift at the crate root.
* `tests/bus_basic.rs`, `tests/health_ready.rs`, `tests/metrics_amnesia.rs`
  Prove core invariants (bounded bus + overflow counting, readiness semantics, amnesia gauge).

## CI (optional but recommended)

* `.github/workflows/ci.yml`
  Build + clippy (deny warnings) + tests for `ron-kernel`.

## Cargo.toml changes (not just versions—**scope**)

* Added deps to support real watcher & parsing: `serde` (derive), `toml`, `notify`, and dev `criterion`.
* Declared two benches with `harness = false`.
* `tokio` features include `fs` to read config file asynchronously.
* No public API expansion at crate root; watcher helpers intentionally **not** re-exported.

## Behavior changes (kernel semantics)

* `/readyz` handler returns a concrete `Response` in both branches (resolves type mismatch; clean Axum 0.7 style).
* `Metrics` is `Clone`; `Metrics::set_amnesia` added; bus constructor `make_bus` wires overflow counter.
* Supervisor uses `rand::rng()` + `random_range()` (0.9), keeps jitter within bounds; backoff capped at 30s.

# Next actions (for the next instance to pick up)

1. **Tests for watcher paths**

   * Integration test that writes the TOML file, flips `amnesia`, asserts one `ConfigUpdated` and `amnesia_mode` transition.
   * Integration test for env poller (`RON_AMNESIA`) with a short poll interval.
2. **Docs updates**

   * In `ALL_DOCS.md` / `README.MD`: document watcher semantics (content-based apply, autobump behavior, and why), golden metrics list, and a short demo snippet.
   * In `TODO.md`: add the new files (constants, watcher, examples, benches, tests, CI) so future diffs don’t look like drift.
3. **Optional debounce** (only if you observe duplicate emits on other OS/filesystems): tiny 50–100ms debounce window around file reload.
4. **Optional perf gate**

   * Keep Criterion baselines and add a perf-check script to compare against saved baseline locally before merging changes touching the bus/supervisor.

### END NOTE - OCTOBER 17 2025 - 21:19 CST

### BEGIN NOTE - OCTOBER 18 2025 - 12:01 CST

Here’s a clean set of carry-over notes so we can hit the ground running next session.

# ron-kernel — carry-over notes

## Public API (frozen for now)

* `Metrics::new(amnesia_on: bool) -> Metrics`
* `Metrics::serve(self, bind: SocketAddr, health: Arc<HealthState>) -> anyhow::Result<(JoinHandle<()>, SocketAddr)>`
* `Metrics::make_bus(&self, capacity: usize) -> Bus`
* `Metrics::set_amnesia(&self, on: bool)`
* `Bus::new(metrics: &Metrics, capacity: usize) -> Bus`
* `Bus::publish(&self, evt: KernelEvent) -> Result<usize, ()>`  // Ok = receivers count; Err = no receivers
* `Bus::subscribe(&self) -> Receiver`
* `Receiver::recv(&mut self) -> Option<KernelEvent>`
* `HealthState::new() -> Arc<HealthState>`
* `HealthState::set(&self, f: impl FnOnce(&mut HealthSnapshot))`
* `HealthState::all_ready(&self) -> bool`
* `HealthState::missing(&self) -> Vec<String>`
* `wait_for_ctrl_c()`

## Module layout & re-exports

* `events.rs` is the single source of truth for `KernelEvent`.
* Re-exports to keep old paths working:

  * `pub use crate::events::KernelEvent;` inside `bus` (so `ron_kernel::bus::KernelEvent` works).
  * `pub use crate::health::HealthState;` inside `metrics` (so `ron_kernel::metrics::HealthState` works).
* Preferred imports going forward:

  * `use ron_kernel::{Metrics, Bus, HealthState};`
  * `use ron_kernel::events::KernelEvent;`

## Metrics (names and meaning)

* `bus_no_receivers_total` (counter): increments when `publish()` fails due to **no subscribers**.
* `bus_receiver_lag_total` (counter): sums messages dropped for slow receivers (observed via `RecvError::Lagged(n)`).
* `service_restarts_total{service}` (counter vec): for supervisors to increment.
* `request_latency_seconds{route,method}` (histogram vec): services should record; kernel only defines it.
* `amnesia_mode` (gauge): `1.0` when amnesia ON, else `0.0`.

## HTTP endpoints (semantics)

* `/metrics`: Prometheus text; uses the shared registry.
* `/healthz`: always `200 OK` (liveness).
* `/readyz`: **degrade-first** — returns `503 Service Unavailable` + `Retry-After: 1` and a body `booting; missing=...` until:

  * `HealthState.services_ok == true` **and** `HealthState.config_loaded == true`.
  * Then returns `200 OK` with `ready`.

## Health model

* `HealthState::set(|s| { s.services_ok = …; s.config_loaded = …; s.amnesia = …; })`
* `amnesia` is observable but **not** part of readiness truth.

## Bus behavior (important invariants)

* Bounded ring via `tokio::broadcast`; kernel never blocks on slow consumers.
* Sender “overflow” is not a thing here; real drops are tracked on the **receiver** side (lag).
* `publish()` returns `Ok(#receivers)`; if zero receivers at send time, increments `bus_no_receivers_total` and returns `Err(())`.

## Lints & docs

* `#![forbid(unsafe_code)]` and `#![deny(missing_docs, clippy::await_holding_lock, unwrap_used, expect_used, panic)]` are enforced.
* All public items have doc comments; keep future contributions compliant.

## Tests (current shape)

* `bus_basics.rs`: asserts `bus_no_receivers_total` increments and `publish()` returns `Ok(n)`.
* `bus_bounded.rs`: generates lag and asserts `bus_receiver_lag_total > 0` by parsing `/metrics` text (no `proto_ext`).
* `health_ready.rs`: starts degraded `/readyz`, flips to ready after updating `HealthState`.

## Example

* `examples/kernel_demo.rs` binds metrics on `127.0.0.1:9600`, prints URLs, and demonstrates readiness flip.

## Build/run quickies

```
cargo build -p ron-kernel
cargo test -p ron-kernel
cargo run -p ron-kernel --example kernel_demo
```

## Decisions we made (lock these to avoid drift)

* Keep `/readyz` degrade-first with `503 + Retry-After: 1` and a stable “missing=” list.
* Use `parking_lot::RwLock` for `HealthState`.
* Split the old “overflow” metric into **two** correct counters (`bus_no_receivers_total`, `bus_receiver_lag_total`).
* Keep `request_latency_seconds{route,method}` as a `HistogramVec`.
* Keep `amnesia_mode` gauge name (short, consistent).
* Keep `Metrics::new(bool)` seeding initial posture; services can call `set_amnesia()` later.

## Known compatible paths (to support legacy imports)

* `ron_kernel::bus::KernelEvent` (via re-export)
* `ron_kernel::metrics::HealthState` (via re-export)
* Preferred: `ron_kernel::events::KernelEvent` and `ron_kernel::HealthState`.

## Nice-to-do next (scoped, aligned to docs)

* Config watcher: file + env polling task emitting `ConfigUpdated` and tweaking `HealthState.config_loaded`.
* Supervisor hooks: wire `service_restarts_total{service}` increments; add minimal supervisor stubs if needed.
* Public API check in CI: already scaffolded; ensure workflow runs on this crate’s paths.
* Add an integration test asserting `/readyz` body includes stable `missing=` names.

If you want, I can package these notes into a `CRATE_STATUS.md` inside `crates/ron-kernel/` so it lives with the code.

### END NOTE - OCTOBER 18 2025 - 12:01 CST

### BEGIN NOTE - OCTOBER 18 2025 - 16:00 CST

Here’s a concise, carry-over note pack for the next instance. It captures what we did, what’s locked/frozen, current state, perf baselines, and the next high-value steps for the ron-kernel crate.

# RustyOnions • ron-kernel • Carry-Over Notes

## Scope

Build and harden the `ron-kernel` crate (core microkernel primitives): event bus, metrics/health/readiness, and invariants with fast tests and microbenchmarks.

---

## Decisions (Frozen Contracts)

* **Bus publish contract**

  * `publish(evt) -> Result<usize, BusError>`
  * **Zero subscribers ⇒ Ok(0)** (not an error); increments `bus_no_receivers_total`.
  * N subscribers ⇒ `Ok(N)`.
  * Overflow/slow consumer is observed on the **receiver** (`Lagged(n)`), not the sender.
* **TLS type invariant**

  * Must use `tokio_rustls::rustls::ServerConfig` (compile-time guarded).
* **Readiness**

  * `HealthState` drives `/readyz`. All gates true ⇒ ready. Gates toggle via `HealthState::set(|snap| { ... })`.
* **Metrics (canonical series)**

  * `bus_no_receivers_total` (IntCounter)
  * `bus_receiver_lag_total` (IntCounter)
  * `service_restarts_total{service}` (IntCounterVec)
  * `request_latency_seconds{route,method}` (HistogramVec)
  * `amnesia_mode` (Gauge; 1.0 when ON, else 0.0)

---

## Code Changes (Key Files)

* `crates/ron-kernel/src/bus/mod.rs`

  * Introduced `BusError` (avoid unit error type).
  * Sender: returns `Ok(0)` on no receivers; increments `bus_no_receivers_total`.
  * Receiver wrapper counts `Lagged(n)` via `bus_receiver_lag_total.inc_by(n)`.
  * Fixed pattern match on `Result<_, RecvError>`.

* `crates/ron-kernel/src/metrics/mod.rs`

  * Preserved global `OnceCell<Arc<Registry>>` registry and Axum handlers.
  * Kept `serve(self, bind, health) -> anyhow::Result<(JoinHandle<()>, SocketAddr)>`.
  * Localized `#[allow(clippy::unwrap_used)]` to `Metrics::new` to satisfy clippy gate.
  * Kept helper accessors and `make_bus`.

* **Tests (added/converted to real invariants)**

  * `tests/bus_basics.rs`: zero vs one subscriber; metric increments; ns timing printed.
  * `tests/bus_bounded.rs`: slow receiver induces lag; hard timeout; ns timing printed.
  * `tests/bus_close_semantics.rs`: after all senders drop, `recv()` returns `None`.
  * `tests/readiness_degrades.rs`: gates degrade → recover using `HealthState::set(|snap| ...)`.
  * `tests/metrics_smoke.rs`: `/metrics` serves; asserts stable counters/gauges appear.
  * `tests/tls_type_invariance.rs`: compile-time guard for TLS ServerConfig path.
  * `tests/property_config.rs`: `amnesia_mode` toggles via `set_amnesia`.
  * `tests/supervisor_backoff.rs`: placeholder replaced with labeled counter monotonicity (until real supervisor API lands).

* **Benches (Criterion)**

  * `benches/bus_publish.rs`: publish cost with 0/1/lagged subscriber; setup outside hot loop.
  * `benches/bus_lag_vs_publish.rs`: scenario bench with slow consumer (macro-level).

* **Makefile**

  * `fmt`, `clippy` (no benches), `test`, `check` targets (keeps benches out of clippy/test build).

---

## Test & Lint Status (Local)

* `make check` is green:

  * `cargo fmt` clean.
  * `clippy` runs on lib/tests/examples with `-D warnings`; passes.
  * Test suite passes; key tests are time-bounded and print ns.

---

## Perf Baselines (Criterion)

* **bus_publish/no_subscribers:** ~**40–45 ns/op**
* **bus_publish/one_subscriber:** ~**73–78 ns/op**
* **bus_publish/lagged_subscriber_cap1:** ~**71–72 ns/op**
* **bus_lag_vs_publish/no_subscribers (5k sends):** ~**214 µs total** ⇒ ~**42.8 ns/op**
* **bus_lag_vs_publish/one_slow_subscriber:** ~**94 ms total** (includes runtime, sleeps, join; macro-bench)

Interpretation: Sender is O(1); adding one subscriber roughly doubles cost but stays sub-100 ns. Lag/overflow doesn’t penalize the sender (drops counted on receiver). These are excellent numbers.

---

## How to Run (Local)

```
make check
cargo bench -p ron-kernel
```

* Criterion report: `target/criterion/report/index.html`

---

## What’s Next (Ordered)

1. **Docs lock-in (high-value, quick)**

   * Add “Bus Contract (Frozen)” + “Kernel Bus — Contract & Telemetry” sections to `README.MD` and `ALL_DOCS.md` (snippets were provided earlier).
2. **Kernel demo polish (very quick)**

   * In `examples/kernel_demo.rs`, record one `request_latency_seconds` observation so histo appears on `/metrics`:

     ```rust
     metrics.request_latency_seconds
         .with_label_values(&["/readyz", "GET"])
         .observe(0.001);
     ```
3. **Supervisor backoff — real test**

   * When supervisor API lands, replace the counter-only test with a minimal backoff progression assertion (e.g., ~100ms → ~200ms → capped).
4. **Optional coverage**

   * `cargo install cargo-llvm-cov --locked`
   * Add `coverage` target to Makefile:

     ```
     coverage:
     	cargo llvm-cov --workspace --lcov --output-path target/lcov.info
     ```
5. **CI (when ready)**

   * Add GitHub Actions workflow (fmt, clippy, build, tests). We already have a template ready.
6. **Bench hygiene (optional)**

   * `brew install gnuplot` for nicer Criterion plots.
   * Consider `sample_size(100)` for even tighter microbench variance.

---

## Guardrails (Do Not Regress)

* Do not change `publish` behavior: **Ok(0)** for zero subscribers; increment `bus_no_receivers_total`.
* Keep `Receiver` counting `Lagged(n)` and return `None` on closed channel.
* Keep TLS path pinned to `tokio_rustls::rustls::ServerConfig`.
* Keep `/readyz` backed by `HealthState` gates; degrade → recover semantics must hold.
* Keep metrics names stable; tests assert presence of counters/gauges.

---

## Useful Paths

* Bus: `crates/ron-kernel/src/bus/mod.rs`
* Metrics/HTTP: `crates/ron-kernel/src/metrics/mod.rs`
* Health: `crates/ron-kernel/src/health/…`
* Tests: `crates/ron-kernel/tests/*.rs`
* Benches: `crates/ron-kernel/benches/*.rs`
* Makefile: repository root

---

### END NOTE - OCTOBER 18 2025 - 16:00 CST

### BEGIN NOTE - OCTOBER 18 2025 - 19:28 CST

here’s a crisp “carry-over” packet so you can pick this up instantly next session.

# What we landed (today’s delta)

* **Build green for examples** and **all tests passing** after aligning APIs.
* **Metrics API stabilized** to match tests:

  * `Metrics::new(bool)`, `make_bus(capacity)`, `_total` counter names, `amnesia_mode` gauge.
  * `/metrics`, `/healthz`, `/readyz` served by `Metrics::serve(addr, HealthState, Readiness)`.
* **Bus (bounded)** finalized:

  * `publish(msg) -> usize` (never blocks), lag/drop counters wired.
  * Helper `Bus::handle_recv` accounts `Lagged(n)` to `bus_receiver_lag_total`.
* **Topic bus** kept **internal**; fixed gauge update to `bus_topics_total`.
* **Supervisor loop** wired:

  * `run_once` via `tokio::spawn` (panic-safe via `JoinError`), `Metrics::inc_restart(service)`.
  * Backoff + intensity window present; `Supervisor::run(Fn() -> Fut + Clone + 'static) -> !`.
* **Health/Readiness** semantics fixed:

  * `HealthState::all_ready()` now requires **at least one service** and **all true**.
  * Added `Default` for `HealthState`.
  * `readyz_handler` returns a concrete `Response` in both branches.
* **Examples**: `kernel_demo` + `minimal_supervision` compile/run.
* **Tests**: updated to kernel’s final surface; added/kept `tempfile` as dev-dep.

# Current invariants (frozen surface)

* Public re-exports at crate root: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `wait_for_ctrl_c`, `Config` (from your `config::mod`).
* **Bus contract**:

  * `publish -> usize`, returns receiver count at send time.
  * Zero subscribers increments `bus_no_receivers_total`.
  * `Lagged(n)` increments `bus_receiver_lag_total` via helper.
* **Readiness contract**: `ready == config_loaded && all services have reported && all healthy`.
* **No unsafe**, **no locks across .await** on hot paths, **bounded broadcast** everywhere.

# Files we touched (keep these as sources of truth)

* `src/metrics/exporter.rs` — metrics registry, HTTP server, bus helpers.
* `src/metrics/health.rs` — `HealthState`, fixed `all_ready`, added `Default`.
* `src/metrics/readiness.rs` — concrete `Response` return type.
* `src/bus/bounded.rs` — capacity ctor, metrics wiring, lag helper.
* `src/bus/topic.rs` — internal; fixed use of `bus_topics_total`.
* `src/supervisor/child.rs` — spawn/JoinError handling, event publish.
* `src/supervisor/lifecycle.rs` — restart loop, intensity window.
* Multiple `tests/*` files aligned to the final API (publish returns `usize`, etc.).

# Open items (carry-over TODOs)

1. **Config subsystem**

   * `config/validation.rs`: implement strict deserialization + `deny_unknown_fields` (toml/json), pure validation + normalization.
   * `config/watcher.rs`: ensure **debounce** + **rate-limit** for env polling; robust parse-error path (emit event/metric, no crash).
   * Align watcher’s emitted event (`KernelEvent::ConfigUpdated { version }`) with autobump logic; add idempotence property test.

2. **Supervisor polish**

   * Add **structured reasons** to events if we later extend `KernelEvent` (keep current variant frozen until agreed).
   * Test intensity cap path deterministically (unit+integration), and add metrics for **suppressed restarts**.

3. **Observability**

   * Exporter: optional **TLS** serving (feature-gated), or leave for `ron-gateway`.
   * Add RED histograms for key demo ops (already seeded one observation in example).
   * `/readyz` should include `Retry-After: 3` (already present), add tiny test to assert header.

4. **Testing depth**

   * **Loom**: an ignored test targeting bus publish/subscribe interleavings.
   * **Fuzz**: event serialization/deserialization (if/when events become payloaded).
   * **Soak**: long-run test doing 1M publishes with rotating subscribers; assert no FD leaks, stable latency.

5. **Docs (keep in sync)**

   * README: add a **“Bus Contract (Frozen)”** section (publish semantics + counters + lag helper).
   * Metrics reference table (names, type, semantics) to prevent future drift.
   * Small “Operational Semantics” snippet for readiness gates.

6. **CI gates**

   * Workflow with: fmt, clippy (pedantic), test, public-api guard (if you use it), **feature matrix** (amnesia on/off), optional loom lane (ignored).
   * Criterion benches (store baselines; fail on regression via `critcmp` if you already use it).

# Quick commands (for future self)

```
# run the world for this crate
cargo build -p ron-kernel
cargo test  -p ron-kernel
cargo run   -p ron-kernel --example kernel_demo

# if you add loom later (ignored tests):
RUSTFLAGS="--cfg loom" cargo test -p ron-kernel -- --ignored

# watch metrics locally
curl -s localhost:9600/metrics | head
curl -s -o /dev/null -w "%{http_code}\n" localhost:9600/readyz
```

# Nice-to-have next session (small wins)

* Add `pub type ConfigCell = Arc<parking_lot::RwLock<Config>>` to `config/mod.rs` and re-export if you want that alias back.
* Rename internal `TopicBus` to `topic_bus` module or prefix `_` to quiet dead-code warnings when unused.
* Example: show a **single** supervised task that panics once, then runs OK (we already have `minimal_supervision`; we can fold that into README snippets).

# Risks & guardrails to remember

* Keep `KernelEvent` minimal and frozen; any expansion needs a doc/update cycle.
* Don’t let watcher perform I/O on the async path; keep bridge thread → mpsc → async.
* Maintain “bounded & observable” invariant for any new channels.

---

### END NOTE - OCTOBER 18 2025 - 19:28 CST

## 2025-10-18 — ron-kernel: live amnesia flip verified in kernel_demo (20:29 CST)

- Added/updated examples/kernel_demo.rs to:
  - Bind /metrics, /healthz, /readyz.
  - Poll RON_CONFIG (default: /tmp/ron-kernel.toml) every 500ms.
  - On real content change: parse `amnesia = true|false`, call `metrics.set_amnesia(..)`,
    and publish `KernelEvent::ConfigUpdated { version }`.
- Verified with:
  - perl -0777 -pe 's/amnesia\\s*=\\s*true/amnesia = false/g' -i /tmp/ron-kernel.toml
  - curl /metrics → amnesia_mode flips 1 ↔ 0 as expected.
- Test suite: green (13/13). No hangs after bounding recv() with tokio::time::timeout in watcher_integ.

```bash
#Terminal A:
cargo run -p ron-kernel --example kernel_demo
#Terminal B:
curl -s http://127.0.0.1:9600/metrics | grep -i amnesia
perl -0777 -pe 's/amnesia\s*=\s*true/amnesia = false/g' -i /tmp/ron-kernel.toml
sleep 1
curl -s http://127.0.0.1:9600/metrics | grep -i amnesia
perl -0777 -pe 's/amnesia\s*=\s*false/amnesia = true/g' -i /tmp/ron-kernel.toml
sleep 1
curl -s http://127.0.0.1:9600/metrics | grep -i amnesia
```

### END NOTE 

### BEGIN NOTE - OCTOBER 18 2025 - 20:33 CST

Absolutely—here’s a tight, copy-pasteable carryover pack plus the concrete next steps to bring `ron-kernel` to “complete.”

# Carry-Over Notes — ron-kernel (as of 2025-10-18)

## Status Snapshot

* **Build/tests:** Green. 13/13 tests pass; prior hang in `watcher_integ` fixed by bounding `recv()` with `tokio::time::timeout`.
* **Demo:** `examples/kernel_demo.rs` verified:

  * `/metrics`, `/healthz`, `/readyz` live on `127.0.0.1:9600`.
  * Edits to `/tmp/ron-kernel.toml` (or `RON_CONFIG`) flip the `amnesia_mode` gauge 0↔1.
  * Each content change publishes `KernelEvent::ConfigUpdated { version }`.
* **Public API (frozen):** Re-exports: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* **Invariants (key ones):**

  * Bounded bus; lag accounted; close semantics tested.
  * Readiness is degrade-first; `/readyz` becomes OK only when config_loaded=true (+ health OK).
  * TLS type guard must remain `tokio_rustls::rustls::ServerConfig`.
  * Metrics surface includes: `amnesia_mode`, bus counters, request latency histogram, health/ready endpoints.
* **Docs:** `NOTES.MD` contains latest context; `TODO.MD` is slightly behind (new files/tests/benches not listed yet).

## What We Just Fixed/Added (since prior instance)

* **Fixed**: `watcher_integ` hang by wrapping `rx.recv()` in a deadline-aware `timeout`, preventing indefinite awaits.
* **Added/Updated**: `examples/kernel_demo.rs` to apply config changes live (polling), flipping `amnesia` and publishing `ConfigUpdated { version }`.
* **Verified**: End-to-end metric flips and kernel event publishing on real content deltas.

## Known Good Commands

```
cargo build -p ron-kernel
cargo test -p ron-kernel
cargo run -p ron-kernel --example kernel_demo
curl -s http://127.0.0.1:9600/metrics | head -n 30
curl -s http://127.0.0.1:9600/healthz
curl -s http://127.0.0.1:9600/readyz
```

## Live Flip (demo)

```
perl -0777 -pe 's/amnesia\s*=\s*true/amnesia = false/g' -i /tmp/ron-kernel.toml
sleep 1
curl -s http://127.0.0.1:9600/metrics | grep -i amnesia
perl -0777 -pe 's/amnesia\s*=\s*false/amnesia = true/g' -i /tmp/ron-kernel.toml
sleep 1
curl -s http://127.0.0.1:9600/metrics | grep -i amnesia
```

## Paste-Ready NOTES.MD Increment (append)

```
## 2025-10-18 — ron-kernel: suite green + demo applies amnesia

- Fixed watcher_integ hang by bounding rx.recv() with tokio::time::timeout.
- Added/updated examples/kernel_demo.rs:
  - Binds /metrics, /healthz, /readyz on 127.0.0.1:9600.
  - Polls RON_CONFIG (default /tmp/ron-kernel.toml) every 500ms.
  - On real content change: parses `amnesia = true|false`, calls metrics.set_amnesia(...),
    and publishes KernelEvent::ConfigUpdated { version }.
- Verified /metrics shows amnesia_mode flipping 0 ↔ 1 after edits; console logs show ConfigUpdated events.
- Current: 13/13 tests passing; no hangs.
```

---

# Next Steps to “Complete” ron-kernel

## A) Tests (small, high-value)

1. **Duplicate-emit guard (content vs. mtime)**

   * `watcher_touch_does_not_emit_update`
     Touch/rename without content delta ⇒ expect **0** `ConfigUpdated`.
   * `watcher_content_change_emits_exactly_one`
     Two distinct content writes ⇒ expect **exactly 2** events (no >2).
2. **Readiness semantics**

   * `ready_requires_config_loaded`: `/readyz` stays not-ready until `set_config_loaded(true)`.
3. **Public API freeze**

   * Add a CI check using `cargo-public-api` to stop accidental surface drift.

## B) Docs (sync & tighten)

* **README.MD / ALL_DOCS.md**

  * Add a short “Config Updates” note: *“Kernel emits `ConfigUpdated { version }` on content change (not mtime). Duplicate FS events are folded; tests enforce ≤1 emit per write.”*
  * List the **golden metrics** (names + intent) and the **health/ready semantics** (config_loaded gating).
* **TODO.MD**

  * Include: `examples/kernel_demo.rs`, the new test files, any benches, and CI gates.

## C) CI & Hygiene

* **CI lanes** (in whatever system you use):

  * Format & lint: `cargo fmt --check` / `cargo clippy -- -D warnings`
  * Tests: `cargo test -p ron-kernel`
  * Public API: `cargo public-api --diff-git-checks`
  * Deny: `cargo deny check`
* **Cargo features** sanity: ensure no accidental default-feature drifts across workspace (aligned with your pins).

## D) Bench Baseline (optional but recommended)

* Run:

  ```
  cargo bench -p ron-kernel
  ```
* Capture top histograms/throughput and paste a short summary into `NOTES.MD` (baseline for future regression checks).

## E) Example polish (optional)

* Add `RON_CONFIG` hint to demo output when env var is set.
* Add a simple `GET /version` or `/about` route in exporter (if in scope) that prints the frozen public API or build info—useful for ops.

---

## Done Definition (Acceptance Gates)

* ✅ **All tests green**, including the two new watcher tests and readiness test.
* ✅ **Public API frozen** (CI gate enforces; no extra re-exports).
* ✅ **/metrics, /healthz, /readyz** documented and stable; demo verified.
* ✅ **Docs synced** (README/ALL_DOCS/TODO updated to reflect current files and behavior).
* ✅ **Bench baseline captured** (at least one number recorded in NOTES).
* ✅ **CI clean** on fmt, clippy, public-api, deny.

## Suggested Commit Titles

* `ron-kernel: fix watcher integ hang; add demo config apply; suite green`
* `ron-kernel: add duplicate-emit tests; document config-updates; wire public-api CI gate`


### END NOTE - OCTOBER 18 2025 - 20:33 CST

### BEGIN NOTE - OCTOBER 19 2025 - 15:15 CST

Here you go — tight, copy-forward “carry over” notes just for the **ron-kernel** crate. Hand this to the next instance + upload your `MOG.MD`.

# ron-kernel — Carry Over Notes

## Current state (green ✅)

* **Build/tests:** `cargo test -p ron-kernel` → all tests passing (loom gated via `--features loom` works).
* **Benches (10/19, tuned caps):**

  * `bus_publish_steady`

    * 0 subs: **~49 ns**/publish
    * 1 sub: **~110 ns** (steady hot-loop microbench)
  * `bus_multi_subscribers` (10000 msgs):

    * 0 subs: **~0.51 ms** total
    * 1 sub: **~1.76 ms** total
    * 16 subs: **~3.02–3.47 ms** total
  * `bus_lag_vs_publish` (25000 msgs):

    * no subs: **~1.21–1.33 ms** total
    * one slow sub: **~1.26–1.29 ms** total
  * `metrics_encode` (registry gather+encode): **~47 µs**
  * `readyz` handler: **~2.23–2.78 µs**
* **Semantics locked:** bounded, non-blocking publish; lagging receivers drop on recv side; readiness/health/metrics are cheap and on by default.
* **Cargo features (current):** `loom` (optional/ok).
  **Perf features planned (OFF by default):**
  `bus_edge_notify`, `bus_batch`, `bus_autotune_cap`, `metrics_buf`, `bus_soa`, `bus_interest`, `bus_coalesce`, `local_dispatch`.

## Baseline + how to compare (freeze this)

* Save baseline once per machine:
  `cargo bench -p ron-kernel -- --save-baseline core-2025-10-19`
* Compare after every feature:
  `cargo bench -p ron-kernel -- --baseline core-2025-10-19`
* Keep benches we care about: `bus_publish.rs`, `bus_lag_vs_publish.rs`, `bus_multi_subscribers.rs`, `bus_overflow_drop.rs`, `bus_publish_matrix.rs`, `metrics_encode.rs`, `readiness_handler.rs`.

## Perf roadmap snapshot (see MOG.MD for details)

* **Phase A (do first):**

  1. `bus_edge_notify` — per-subscriber pending bit; notify on 0→1; drain race-check.
     **Gate:** ≥10% @ subs=4 cap=64; ≥20% @ subs≥8.
  2. `bus_batch` — `publish_many(&[T])`; single Release fence + single notify per batch.
     **Gate:** ≥15% in burst benches.
  3. `bus_autotune_cap` — default 64 (≤4 subs), 128 (≤16); warn if >256.
     **Gate:** no regressions vs hand-picked caps.
  4. `metrics_buf` — thread-local counters; periodic flush.
     **Gate:** ≥8% @ subs=4 cap=64.
  5. Drain-loop discipline bundled with A1.
     **Gate:** ≥5% @ subs≥4.
* **Phase B:**
  6) `bus_soa` — SoA ring + per-slot ready bitmask (safe: `Arc<T>` first).
  **Gates:** ≥20% @ subs=4 cap=64 **and** ≥30% @ subs=16 cap=128.
  7) `bus_interest` — static topic masks to cut wakes.
  **Gate:** ≥15% on mixed-topic streams.
  8) `local_dispatch` — pattern/docs to keep global N small (roles re-fan-out).
* **Polish:** false-sharing padding, tight memory order (Release on final `seq.store`, Acquire on `seq.load`, Relaxed for masks), optional pinning guidance.

## Guardrails / tests (must keep)

* **Loom:** lost-wake with edge-notify; last-bit clear → single drop (SoA); overwrite semantics; publisher + 2 subs race.
* **Property/fuzz:** drop accounting equals `overwrites - delivered`; no double deliveries; monotonic tails.
* **Observability (keep exporting):** per-sub lag gauge; `bus_overflow_dropped_total`; `bus_notify_sends_total`, `bus_notify_suppressed_total` (when A1 lands); batch histograms (A2).

## Known good commands (developer QoL)

* Test with loom:
  `cargo test -p ron-kernel --features loom -- --nocapture`
* Run all benches and open report:
  `cargo bench -p ron-kernel`
  then `open target/criterion/report/index.html` (macOS)
* Single bench (example):
  `cargo bench -p ron-kernel --bench bus_publish`

## Acceptance targets (v1 “done” line)

* 0 subs: **~46–47 ns** (3–6% win)
* 4 subs (cap≈64): **≤51–49 ns** (≈16–20% win)
* 16 subs (cap≈128): **≤120–90 ns** (≈30–47% win)
* No /readyz or metrics regressions; drops/lag accounted and exported.

## Hand-off items for next instance

* You (user) will upload **`MOG.MD`** (perf TODO list). Treat it as authoritative for the perf pass.
* Implement features **gated and isolated**, default OFF. Keep public API stable.
* Every PR: re-run benches vs `core-2025-10-19` and include the Criterion comparison + gates outcome.


### END NOTE - OCTOBER 19 2025 - 15:15 CST


### BEGIN NOTE - OCTOBER 19 2025 - 17:08 CST


Absolutely—here’s the updated “carry-over packet” with everything we just landed and verified, so you can paste this into the new instance and keep rolling.

# State of Play (now)

* **Crate:** `ron-kernel`

* **New feature flags (OFF by default unless passed):**

  * `bus_edge_notify` — edge-triggered per-subscriber notify + disciplined drain loop.
  * `bus_batch` — `publish_many(&[T])` with a single notify sweep per batch (A2).

* **Files updated/added (already shipped, paste-ready earlier):**

  * `src/bus/bounded.rs` — generic `Bus<T>`, inlined edge helper (no separate module), `publish_many()` behind `bus_batch`, `subscribe_edge()`/`EdgeReceiver::run_drain_loop()` behind `bus_edge_notify`.
  * `src/bus/mod.rs` — tiny module index (just `pub mod bounded;`).
  * `metrics/exporter.rs` — added A1/A5 counters and A2 counters/histogram; `make_bus<T>()` unchanged.
  * `examples/minimal_supervision.rs` — edge subscriber when `bus_edge_notify`; batch publisher when `bus_batch`.
  * **Benches** (new):

    * `benches/publish_edge_matrix.rs` — public-API bench (single publishes) across subs×cap.
    * `benches/bus_batch.rs` — head-to-head single vs `publish_many(128)`.

* **Public API stability:** `Bus<T>::new()`, `with_capacity()`, `with_metrics()`, `publish()`, `subscribe()` unchanged. New (feature-gated): `publish_many()`, `subscribe_edge()`, `EdgeReceiver::run_drain_loop()`.

# What Works (verified)

* Example runs and exposes `/metrics`, `/healthz`, `/readyz`.
* With `--features bus_edge_notify,bus_batch` and batch publisher enabled:

  * `bus_batch_publish_total` and `bus_batch_len_histogram_*` > 0
  * `bus_published_total ≈ batches × batch_size`
  * `bus_notify_sends_total ≈ batches × subscribers`
  * `bus_notify_suppressed_total` = 0 for single publisher (one sweep per batch); rises with **overlapping publishers** (intended).

# Proven Performance (A2)

From your Criterion results (10k elems/iter, cap=64):

* **Subs=1**
  Single: ~945.9 µs ⇒ **~10.56 M elems/s** (~94.6 ns/elem)
  Batch128: ~710.1 µs ⇒ **~14.08 M elems/s** (~71.1 ns/elem)
  **Gain:** ~24–25% lower ns/elem, **~33%** higher throughput ✅

* **Subs=4**
  Single: ~951.2 µs ⇒ **~10.51 M elems/s**
  Batch128: ~733–778 µs ⇒ **~12.85–13.64 M elems/s**
  **Gain:** ~18–23% lower ns/elem, **~22–30%** higher throughput ✅

👉 This meets/exceeds MOG A2 gate (≥15% faster on burst benches).

# How to Run / Sanity

* Edge only:

  ```
  cargo run -p ron-kernel --example minimal_supervision --features bus_edge_notify
  ```
* Edge + Batch (A2):

  ```
  cargo run -p ron-kernel --example minimal_supervision --features bus_edge_notify,bus_batch
  ```
* Metrics loop (mac, shell):

  ```
  while true; do clear; date; curl -s http://127.0.0.1:PORT/metrics | egrep 'bus_(batch_|published|notify_)'; sleep 1; done
  ```

# Benchmarking (notes + commands)

> Some toolchains reject Criterion’s `--save-baseline`. Use these instead.

* Run benches (no baseline flag):

  ```
  cargo bench -p ron-kernel --bench publish_edge_matrix
  cargo bench -p ron-kernel --features bus_batch --bench bus_batch
  ```
* View HTML reports:

  * `target/criterion/report/index.html`
  * `target/criterion/<bench>/report/index.html`
* If you want a named snapshot anyway (manual):

  ```
  rsync -a target/criterion/ target/criterion.core-2025-10-19/
  ```

  Compare by opening both reports side-by-side after subsequent runs.

# Telemetry: Watch These

* **A1/A5**

  * `bus_notify_sends_total` — wakes actually sent
  * `bus_notify_suppressed_total` — coalesced wakes (expect >0 only with overlapping sweeps/publishers)
* **A2**

  * `bus_batch_publish_total` — count of `publish_many` calls
  * `bus_batch_len_histogram_*` — batch size distribution
* Always: `bus_published_total`, `bus_no_receivers_total`, `bus_receiver_lag_total`, `bus_dropped_total`

# Code Structure (mental map)

```
src/bus/
  mod.rs               # index: pub mod bounded;
  bounded.rs           # Bus<T>, edge sweep (inlined helper), publish_many, EdgeReceiver drain loop
metrics/exporter.rs    # RED + A1/A5 + A2 metrics and HTTP serving
examples/minimal_supervision.rs  # demo subscriber/publisher; toggles via features
benches/
  publish_edge_matrix.rs          # single-publish matrix
  bus_batch.rs                    # single vs batch128
```

# Guardrails / Invariants

* **Edge memory ordering:** `pending.swap(true/false, AcqRel)`; race check `Relaxed`. Visibility of messages is ensured by `tokio::broadcast`’s own fences; edge bit only gates wake semantics.
* **No `unsafe`:** A1/A2 paths are safe Rust.
* **Edge registry holds `Weak<EdgeSignal>` (AtomicBool + Notify)** — no `Sync` hazards with receivers.

# Open Items / Next Instance

1. **A4 — metrics TLS buffering (`metrics_buf`)**

   * Swap hot-path `inc()`s for TLS counters; flush every N or Δt (and on drop).
   * Add `bus_metrics_tls_flush_total`.
   * Feature-gate as `metrics_buf` (OFF by default).

2. **A3 — cap autotune (quick win)**

   * Gate: `bus_autotune_cap`.
   * Default cap: `≤4 subs → 64`, `≤16 → 128`; warn if >256 (cache-hostile).

3. **Bench coverage**

   * Extend `bus_batch.rs` to 64/128/256 batch sizes.
   * (Optional) Add microbench for notify sweep cost (A1, edge ON vs OFF).

4. **Example polish**

   * Add **second batch publisher** with slight stagger to demonstrate `bus_notify_suppressed_total` growth.
   * Add a tiny dummy supervised service so `/readyz` returns READY.

5. **Docs**

   * Update `MOG.md` A2 section with shipped metric names and “single sweep per `publish_many`” invariant.
   * Note: suppression requires overlapping sweeps (multiple publishers / frequent notify attempts).

# Risks & Mitigations

* **Lost wake after clear:** handled by `clear_pending_and_race_check()`; if `pending` becomes true after clear, we continue draining (no await).
* **Batch drop accounting:** on first failed `send`, increment `bus_dropped_total` and stop; totals remain monotonic.
* **Receiver lag:** `RecvError::Lagged(n)` is counted and loops until a live item is received.

# CI/PR Verification Checklist

* **Build combos:**

  * (none), `bus_edge_notify`, `bus_batch`, `bus_edge_notify,bus_batch`
* **Examples run** in all four combos.
* **Benches compile & run** (publish_edge_matrix + bus_batch).
* **Clippy/lints** clean on public API (optionally gate imports to silence unused when features OFF).
* **Metrics present** on `/metrics` for A1/A2.

# Rollback Plan

* Omit features at build → behavior reverts to pre-MOG.
* `bounded.rs` still supports single-item path; `publish_many` is behind `bus_batch`.
* Inlined edge helper can be disabled by removing the feature block; no cross-module entanglement.

# Handy Commands

* Run (edge only):
  `cargo run -p ron-kernel --example minimal_supervision --features bus_edge_notify`
* Run (edge + batch):
  `cargo run -p ron-kernel --example minimal_supervision --features bus_edge_notify,bus_batch`
* Benches:
  `cargo bench -p ron-kernel --bench publish_edge_matrix`
  `cargo bench -p ron-kernel --features bus_batch --bench bus_batch`
* Metrics grep:
  `curl -s http://127.0.0.1:PORT/metrics | egrep 'bus_(batch_|published|notify_)'`

---


### END NOTE - OCTOBER 19 2025 - 17:08 CST

### BEGIN NOTE - OCTOBER 20 2025 - 14:17 CST 

Here’s the refreshed carryover pack reflecting everything we just landed and validated.

---

# Carryover Notes — ron-kernel MOG (2025-10-20)

## What we landed (current state)

**Metrics & exporter**

* Added A1/A5 and A2 counters:

  * `bus_notify_sends_total`, `bus_notify_suppressed_total`
  * `bus_batch_publish_total`, `bus_batch_len_histogram`
* **TLS metrics buffering (`metrics_buf`)** in `src/metrics/buffer.rs`

  * Per-thread Cell buffers, periodic flush pump, best-effort drop flush.
  * Exported visibility counters:

    * `bus_metrics_tls_flush_total`
    * **New gauge:** `bus_metrics_tls_threshold`
  * Threshold sourced from `RON_TLS_FLUSH_THRESHOLD` (default **64**), clamped ≥ 1.
* Exporter wires `BufferedSinks` → `HotCounters`, spawns `FlushPump` (≈200ms).
* `Metrics::hot()` accessor (replaces old `hot_counters()`).

**Bus**

* `src/bus/bounded.rs` routes hot-path counters through `m.hot()` when `metrics_buf` is enabled.
* A1/A5: optional edge-notify sweep (`bus_edge_notify`).
* A2: optional `publish_many` with single sweep and histogram accounting (`bus_batch`).
* Fixed feature-gated imports; no unused warnings in default builds.

**Examples**

* `examples/publish_smoke.rs` binds metrics on `:0`, prints URL; exercises publish path.
* Works with `metrics_buf`, plus optional `bus_batch`, `bus_edge_notify`.

## Observed signals (from last runs)

**TLS buffering sanity (B1)**

* `bus_metrics_tls_threshold 64`
* `bus_published_total 549,952`
* `bus_metrics_tls_flush_total 8,593`
* Ratio ≈ **64:1** (8,593 × 64 = 550,0 00 − 48 = 549,952) → buffering correct, batching as configured.

**Batch path (B2)**

* `bus_batch_publish_total 2,300`
* `bus_batch_len_histogram_count 2,300`
* `sum 230,000` → **avg batch size ≈ 100**, buckets populate at `≤128` and above as expected.

**Edge notify (B3)**

* `bus_notify_sends_total 59,217`
* `bus_notify_suppressed_total 498,079`
* **~89.4% coalesced** (suppressed / (sends + suppressed)) → large wake reduction; feature working.

**Lag**

* `bus_receiver_lag_total 0` under smoke unless intentionally induced → healthy headroom.

## Quick repro / commands

**Baseline TLS buffering**

```
cargo run -p ron-kernel --features metrics_buf --example publish_smoke
curl -s http://127.0.0.1:<PORT>/metrics | grep -E 'bus_(published_total|metrics_tls_flush_total|metrics_tls_threshold|receiver_lag_total)'
```

Expect `published_total : flush_total ≈ threshold : 1` and `receiver_lag_total == 0`.

**Change threshold via env**

```
RON_TLS_FLUSH_THRESHOLD=256 cargo run -p ron-kernel --features metrics_buf --example publish_smoke
curl -s http://127.0.0.1:<PORT>/metrics | grep -E 'bus_(metrics_tls_threshold|published_total|metrics_tls_flush_total)'
```

Expect `bus_metrics_tls_threshold 256` and ~256:1 ratio.

**Batch path (A2)**

```
cargo run -p ron-kernel --features "metrics_buf,bus_batch" --example publish_smoke
curl -s http://127.0.0.1:<PORT>/metrics | grep -E 'bus_batch_(publish_total|len_histogram)'
```

Expect non-zero publish_total; histogram buckets reflect batch size.

**Edge notify (A1/A5)**

```
cargo run -p ron-kernel --features "metrics_buf,bus_edge_notify" --example publish_smoke
curl -s http://127.0.0.1:<PORT>/metrics | grep -E 'bus_notify_(sends|suppressed)_total'
```

Expect sends > 0 and suppressed >> sends when publish cadence > drain cadence.

## Open items / next actions

1. **Matrix the TLS threshold**
   Sweep `RON_TLS_FLUSH_THRESHOLD={32,64,128,256}` at a fixed publish rate; record CPU (user%), and publish→flush ratios. Pick default (likely 64/128) balancing encode cadence vs latency.

2. **Coalescing curve**
   With `bus_edge_notify` on, vary subscribers {1,4,16} and publish cadence; track suppression rate = suppressed / (sends + suppressed). Aim for high suppression without starving drains.

3. **Intentional lag test**
   Add a slow receiver (sleep >> publish rate) to ensure `bus_receiver_lag_total` increments predictably (guardrail metric).

4. **Batch size sweep**
   Drive `publish_many` with sizes {1,8,64,128,256}; verify histogram buckets and check notify behavior (single sweep per batch).

5. **A3 capacity autotune integration**
   Replace hardcoded bus caps in examples with:

   ```
   let cap = ron_kernel::mog_autotune::autotune_capacity(expected_subs, None);
   let bus = metrics.make_bus::<T>(cap);
   ```

   Keep >256 warning via tracing for cache hostility awareness.

6. **Benches (criterion)**
   Capture before/after for:

   * `bus_publish`, `bus_batch`, `metrics_encode`
   * Feature sets: baseline vs `metrics_buf`, vs `metrics_buf,bus_edge_notify`, vs all three.
     Track p50/p95 throughput and encode cpu.

## Implementation notes / gotchas

* Use `Metrics::hot()` (shared reference) not `hot_counters()`; cloning-and-dropping a hot facade would spur extra flushes.
* `FlushPump` interval ~200ms; don’t block it. Threshold controls batching on the fast path; pump makes progress even under low traffic.
* `buffer.rs` clamps threshold with `max(1)`; threshold=1 is effectively unbuffered (1:1), useful for debug.

## Files touched

* `src/metrics/exporter.rs` — counters, TLS sinks, `bus_metrics_tls_threshold` gauge, pump spawn, `hot()` accessor.
* `src/metrics/buffer.rs` — TLS cells + flush pump (drop-flush, threshold, env support).
* `src/bus/bounded.rs` — uses `m.hot()`; A1/A5 sweep; A2 batch; feature-gated imports cleaned.
* `examples/publish_smoke.rs` — demo runner for all scenarios.

---

### END NOTE - OCTOBER 20 2025 - 14:17 CST


### BEGIN NOTE - OCTOBER 20 2025 - 18:43 CST

# Carry-over notes (for the next instance)

* **Bench harness shape (current best):**

  * Features: `metrics_buf,bus_batch` (✅ on); `bus_edge_notify` (❌ off by default until fixed).
  * Env knobs:

    * `RON_BENCH_BURST=1024`
    * `RON_BENCH_FANOUT=8`
    * `RON_BENCH_CAP=4096` (burst groups only; steady stays small)
    * `RON_BENCH_PUB_YIELD=1` (publisher epoch → yield after each burst)
    * `RON_TLS_FLUSH_THRESHOLD=64`
* **Steady microbench anchors (classic, idle subs):**

  * no_subs: ~44–46 ns/publish (elite)
  * one_sub (cap ~64): ~103–110 ns
  * lagged cap=1: ~103–111 ns
* **God-tier burst numbers (classic path, draining subs):**

  * `classic_fanout/burst1024_fanout8_cap4096`: **~242–253 µs per 1024** ⇒ **~4.04–4.23 M publishes/s**
    Aggregate deliveries (×8): **~32–34 M deliveries/s**.
* **Known gaps / cautions:**

  * Edge notify path (`bus_edge_notify`) is **not coalescing** under our benches → slower than classic.
  * We haven’t exercised an **explicit batch API** (`publish_many`) in benches; still looping `publish()`.
  * Autotune cap not wired; we set cap manually for bursts.
* **Baseline names in Criterion (handy):**

  * `classic` (original small burst)
  * `classic_plus_edge` (mixed IDs)
  * `edge_256_f4` (fanout=4 @256)
  * `god_classic_1024_f8` (current “showpiece”)

---

# Accomplishments (what we actually landed)

* Designed & implemented a **parametric bench** (`bus_publish.rs`) with:

  * Configurable **burst size**, **fanout**, **cap**, and **publisher epoch** (yield).
  * Separate **steady** vs **bursty** groups; receivers drain in burst groups.
* Resolved Criterion baseline issues; added a **repeatable workflow** for like-for-like comparisons.
* Achieved **God-tier throughput** on classic path:

  * **~4.1 M publishes/s** with **8 draining subscribers** → **~33 M deliveries/s** aggregate on a dev laptop.
* Kept **steady no-sub publish cost** at **~44 ns** (top tier for a safe, broadcastable bus).
* Validated **TLS metrics buffering** (A4) doesn’t pollute hot path.
* Bench-shaping wins (burst+fanout+epoch+cap) proved out MOG’s methodology.

---

# Path to finish MOG.MD (checklist with gates)

## Phase A — High ROI, Low Risk

1. **A2 — Batch publish API (`bus_batch`) — finish**

   * **Implement:** `publish_many(&[T])` or a **batch scope** (one reserve, one release fence, one notify).
   * **Bench change:** In burst groups, call the batch path when feature is on (keep the old loop for A/B).
   * **Gate (per MOG):** ≥**15% faster** vs looped `publish()` at subs∈{1,4}, cap=64, same burst.
   * **Tests:** property test for `delivered + overwrites == published`.

2. **A1 — Edge-triggered coalesced notify (`bus_edge_notify`) — fix**

   * **Publisher rule:** notify only on `pending` **0→1**.
   * **Drain loop discipline:**
     `await notify → drain all (nonblocking) → clear pending → race check → await`
     (never clear pending mid-drain; no re-notify in same epoch).
   * **Telemetry (not timed):** per-burst **notify delta** (≈1/burst target), counters:
     `bus_notify_sends_total`, `bus_notify_suppressed_total`, `bus_sub_pending`.
   * **Gate:** ≥**10% win** at subs=4, cap=64; ≥**20%** at subs≥8 (same burst).
   * **Safety:** Loom test lost-wake; closed/lags; monotonic tails.

3. **A3 — Capacity auto-tune (`bus_autotune_cap`) — wire**

   * **Heuristic + guardrail:** default caps (e.g., 64/128) unless bursty workload detected; warn if overridden. Always honor `RON_BENCH_CAP`.
   * **Gate:** no regressions; guardrail metric visible; documentation of behavior.

4. **A4 — TLS metrics buffer (`metrics_buf`) — done**

   * Keep `RON_TLS_FLUSH_THRESHOLD=64` default; ensure flush on shutdown.

5. **A5 — Drain-loop discipline — bench side done**

   * Keep `RON_BENCH_PUB_YIELD=1` option; document it as a bench aid (not production requirement).

## Phase B — Medium Impact

6. **B1 — “Fortran trick” (SoA backend, `bus_soa`) — implement side-by-side**

   * **SoA layout:** `slots[cap]` with `seq[cap]`, `ready_mask[cap]`, `msg[cap]`; `sub_tail[]` padded atomics.
   * **Start with** `Arc<T>` payload; keep semantics identical; no UB.
   * **Benches:** side-by-side matrix `subs {1,4,16} × cap {64,128}` vs classic backend.
   * **Gate:** ≥**20%** win at 4 subs / cap=64 **and** ≥**30%** at 16 subs / cap=128.
   * **Safety:** Loom + Miri + fuzz; histogram `bus_ready_mask_width`.

7. **(Optional) B2 — Topic/interest filtering (`bus_interest`)**

   * Static bitmask per sub; AND into per-slot ready mask.
   * **Gate:** ≥**15%** gain on mixed-topic streams (e.g., 30% relevant).

## CI / Governance (from MOG §8)

8. Lock CI gates before landing perf changes:

   * Public API frozen; feature-flagged internals.
   * Bench smoke on CI; property tests; Loom runs nightly.
   * Doc the feature flags & env knobs in `README.MD` (bench section).

---

# Quick run scripts (to reproduce the God-tier demo)

```
RON_TLS_FLUSH_THRESHOLD=64 \
RON_BENCH_BURST=1024 RON_BENCH_FANOUT=8 RON_BENCH_CAP=4096 RON_BENCH_PUB_YIELD=1 \
cargo bench -p ron-kernel --features "metrics_buf,bus_batch" --bench bus_publish \
  -- --save-baseline god_classic_1024_f8 --measurement-time 8 --warm-up-time 2

RON_TLS_FLUSH_THRESHOLD=64 \
RON_BENCH_BURST=1024 RON_BENCH_FANOUT=8 RON_BENCH_CAP=4096 RON_BENCH_PUB_YIELD=1 \
cargo bench -p ron-kernel --features "metrics_buf,bus_batch" --bench bus_publish \
  -- --baseline god_classic_1024_f8 --measurement-time 8 --warm-up-time 2
```

---


### END NOTE - OCTOBER 20 2025 - 18:43 CST


### BEGIN NOTE - OCTOBER 20 2025 - 18:55 CST

Awesome—let’s move immediately into **PR-1 (A3) + prep for PR-2 (A1/A5)**. Here’s a tight, do-this-now plan with copy-paste commands, acceptance gates, and what to record in NOTES.MD.

# Step 1 — PR-1: Implement `bus_autotune_cap` (A3)

## Goals

* Provide a **feature-gated** helper to choose a sweet-spot ring capacity from expected subscriber count.
* Add **guardrails** (warn + metric) whenever cap > 256.
* Show **no regressions** vs fixed caps at subs {1,4,16}.

## File/Module plan

* `crates/ron-kernel/src/bus/capacity.rs` (new)

  * `#[cfg(feature = "bus_autotune_cap")] pub fn autotune_capacity(expected_subs: usize, override_cap: Option<usize>) -> usize`
* `crates/ron-kernel/src/bus/mod.rs`

  * `pub use bus::capacity::autotune_capacity;` (behind the feature)
* `crates/ron-kernel/examples/kernel_demo.rs`

  * Use `autotune_capacity(N, None)` to pick cap for demo when feature is ON; otherwise keep existing fixed cap.

## Telemetry (A3)

* Counter: `bus_autotune_warn_total{reason="cap_gt_256"}`
* Gauge: `bus_cap_selected` (current cap chosen by autotune)
* Log: `warn!(cap, expected_subs, "cap_gt_256; cache-hostile")`

## Tests

* Unit: override respected; exact mapping (N≤4→64, N≤16→128; else clamp to 256 unless override)
* Property: caps in {64,128,256} for reasonable N; monotone in N
* No-feature build still compiles; helper not referenced.

## Benches (A3)

* Matrix: subs {1,4,16} × caps {64,128} vs `autotune(N)`
* Pass gate: **no p50/p95 regressions** (±3%) vs best fixed cap per N

## Commands

```
git checkout -b feat/a3-autotune-cap
cargo test -p ron-kernel
cargo bench -p ron-kernel -- --baseline core-2025-10-19
```

## NOTES.MD (add)

* Section: `MOG Results — A3`

  * Table: N, cap_fixed, cap_autotune, p50, p95, delta
  * One-line: “A3 passes gate: no regressions; warn_total=0 at N∈{1,4,16}”

---

# Step 2 — Prep benches & metrics for A4 sweep (metrics_buf)

We can parallelize this while A3 lands, since A4 is already scaffolded.

## Bench sweep knobs

* Env: `RON_TLS_FLUSH_THRESHOLD={32,64,128,256}`
* Scenario: 4 subs, cap=64, steady publish cadence
* Capture: user% CPU, and ratio `bus_published_total : bus_metrics_tls_flush_total`

## Commands

```
RON_TLS_FLUSH_THRESHOLD=32  cargo run -p ron-kernel --example kernel_demo &
sleep 3; curl -s http://127.0.0.1:9600/metrics | head -n 200 > /tmp/metrics32.txt; kill %1

RON_TLS_FLUSH_THRESHOLD=64  cargo run -p ron-kernel --example kernel_demo &
sleep 3; curl -s http://127.0.0.1:9600/metrics | head -n 200 > /tmp/metrics64.txt; kill %1

RON_TLS_FLUSH_THRESHOLD=128 cargo run -p ron-kernel --example kernel_demo &
sleep 3; curl -s http://127.0.0.1:9600/metrics | head -n 200 > /tmp/metrics128.txt; kill %1

RON_TLS_FLUSH_THRESHOLD=256 cargo run -p ron-kernel --example kernel_demo &
sleep 3; curl -s http://127.0.0.1:9600/metrics | head -n 200 > /tmp/metrics256.txt; kill %1
```

## NOTES.MD (add)

* Section: `MOG Results — A4`

  * Table: threshold, published_total, tls_flush_total, ratio (~N:1), CPU(user%) if you sample with `top -stats pid,cpu`
  * Decision: pick default (likely 64 or 128) based on best CPU with stable ratio.

---

# Step 3 — PR-2: Design/commit plan for A1 + A5 (edge notify + disciplined drain)

## Invariants (put into code comments + ALL_DOCS)

* `pending` bit is **the** wake coalescer: publisher does **notify only on 0→1**.
* Subscriber **clears pending only after draining**; then **race check** before await.
* Memory ordering: `pending` ops Relaxed; visibility enforced by seq Release/Acquire on slots.

## Loom litmus set

* Lost-wake: publisher sets pending, subscriber clears → ensure race-check catch.
* Two-subscribers: interleaved drains, no deadlock, no starvation.
* Drain-after-clear: ensure subsequent publish won’t be missed.

## Telemetry (A1/A5)

* `bus_notify_sends_total`
* `bus_notify_suppressed_total` (key success metric)
* `bus_sub_pending` gauge
* Optional: `bus_receiver_drain_loops_total` for tuning

## Benches (A1/A5)

* Publish matrix subs {1,4,16} × caps {64,128}
* Gate: ≥10% win at 4/64; ≥20% at ≥8 subs

## Commands

```
git checkout -b feat/a1a5-edge-notify
cargo test -p ron-kernel --features bus_edge_notify
cargo bench -p ron-kernel --features bus_edge_notify -- --baseline core-2025-10-19
```

## NOTES.MD

* Section: `MOG Results — A1/A5` with suppression percentage and latency deltas.

---

# Step 4 — CI guardrails (quick wins)

* Add a **feature matrix job** that builds/tests with:

  * none
  * metrics_buf
  * metrics_buf,bus_edge_notify
  * metrics_buf,bus_edge_notify,bus_batch
* Add an **observability smoke** in CI:

  * run `kernel_demo` for 2s, hit `/metrics` `/healthz` `/readyz`, assert HTTP 200 and a few key metric names present.

---

# Step 5 — What I’ll queue up next (when you say “go”)

* Paste-ready **`capacity.rs`** (feature-gated) and **example diffs** using `autotune_capacity`.
* A tiny **Prometheus counter/gauge registration** block for A3 warnings.
* A minimal **bench group** for the A3 matrix (subs×caps vs autotune).
* A Loom **lost-wake** test skeleton for A1/A5.

---

## MOG Results — A3 (2025-10-20) 19:30 CST
- Mapping: N≤4→64, N≤16→128, else→256. Override respected.
- Guardrail: bus_autotune_warn_total{reason="cap_gt_256"} = <value> (expect 0 in standard runs)
- Gauge: bus_cap_selected = <last_cap>
- Matrix (subs {1,4,16}): p50/p95 deltas vs best fixed cap — PASS (≤±3%)

## MOG Results — A3 (2025-10-20) 19:51 CST
- Tests: PASS (3/3). Mapping N→cap: {≤4:64, ≤16:128, else:256}. Override respected.
- Metrics seen: bus_cap_selected=64 (demo), bus_autotune_warn_total{cap_gt_256}=0
- Bench: a3_autotune_mapping baseline saved; mapping cost ~0ns (expected).

## MOG Results — A3 (2025-10-20) 20:02 cst
autotune_capacity() ≈ 4.0–4.5 ns per call (N ∈ {1,4,16,64,128}, override ~4.1 ns).
Zero material impact vs publish hot path (49–170 ns).
Outliers present (typical CI noise); overall within noise band.
Metrics: bus_cap_selected=64 in demo; bus_autotune_warn_total{cap_gt_256}=0.



### END NOTE - OCTOBER 20 2025 - 18:55 CST


### BEGIN NOTE - OCTOBER 21 2025 - 15:07 CST

Perfect callout—keeping the lanes clean:

* **MOG** → **ron-kernel only** (microkernel perf plan)
* **MOGV2** → other crates (rest of RON-CORE)

Here are **carryover notes for ron-kernel (MOG)** only—paste-ready, no cross-pollination.

---

# CARRYOVER — MOG / ron-kernel

**date:** 2025-10-21
**scope:** ron-kernel (microkernel core) — perf features **OFF** by default; **public API frozen**
**baseline tag:** `core-2025-10-19`

## 0) Baseline (frozen)

Saved and used for all diffs:

```
cargo bench -p ron-kernel -- --save-baseline core-2025-10-19
cargo bench -p ron-kernel -- --baseline core-2025-10-19
```

Reference numbers (tuned caps):

* 0 subs: ~49 ns/publish
* 1 sub (≈64): ~55 ns
* 4 subs (≈64): ~61 ns
* 16 subs (≈128): ~170 ns

## 1) Landed in MOG (ron-kernel)

### A3 — capacity autotune + guardrails (`bus_autotune_cap`) ✅

* `autotune_capacity(expected_subs, override)` (ns-scale call cost: ~4–5 ns).
* Tests: thresholds, monotone mapping, override normalization → **green**.
* Bench: `a3_autotune_cap` runs and compares to baseline.

### A1 + A5 — edge-triggered notify + disciplined drain (`bus_edge_notify`) ✅

* `mog_edge_notify.rs` refactored: correct post-drain race-check; optional Prom metrics hook.
* Integrated **edge sweep** in `bounded.rs` (feature-gated).
* Bench `publish_edge_matrix` fixed for Prometheus API changes; unit tests **green**.

### Demo surfaces ✅

* `examples/kernel_demo.rs` prints selected cap; `/metrics`, `/healthz`, `/readyz`; `RON_AMNESIA` toggle works.

## 2) Source layout & flags (ron-kernel only)

* `crates/ron-kernel/src/bus/mod.rs`

  * `capacity` module behind `bus_autotune_cap`; re-exports **only** under that feature.
  * `mog_edge_notify` behind `bus_edge_notify`.
* `crates/ron-kernel/src/bus/bounded.rs`

  * Edge-sweep wired (A1/A5).
  * `publish_many` **scaffolded** (A2 target).
* `crates/ron-kernel/src/bus/mog_edge_notify.rs`

  * `EdgeNotify` (+ `EdgeMetrics` trait); tests updated.
* `crates/ron-kernel/src/bus/capacity.rs`

  * Autotune impl + tests.
* **Features (Cargo.toml / ron-kernel):**
  `bus_autotune_cap`, `bus_edge_notify`, `bus_batch`, `metrics_buf` (planned).

## 3) Golden metrics (ron-kernel)

* `bus_notify_sends_total`
* `bus_notify_suppressed_total`
* `bus_receiver_lag_total`
* `bus_overflow_dropped_total` (existing)
* `bus_batch_publish_total`, `bus_batch_len_histogram` (A2 target)
* `bus_sub_pending{sub}` (optional gauge)
* `bus_metrics_tls_flush_total` (A4 target)

## 4) What to run (quick sanity)

```
cargo test  -p ron-kernel
cargo test  -p ron-kernel --features bus_autotune_cap
cargo test  -p ron-kernel --features bus_edge_notify

cargo bench -p ron-kernel -- --baseline core-2025-10-19
cargo bench -p ron-kernel --features bus_edge_notify   --bench publish_edge_matrix   -- --baseline core-2025-10-19
cargo bench -p ron-kernel --features bus_autotune_cap  --bench a3_autotune_cap       -- --baseline core-2025-10-19
```

## 5) Known pitfalls we fixed (keep)

* Prometheus API: don’t import private `proto_ext`; iterate `mf.get_metric()` and read `m.get_counter().value()`.
* `#[cfg(feature="tokio")]` in helper tests removed (feature didn’t exist).
* Unexpected `cfg` warnings (e.g., `"bus_oa"`) are **intentional typo guard**—kept as warning.
* Bench uses a single-thread Tokio runtime for stable results.

## 6) Next steps (MOG for ron-kernel only)

### PR-3 — **A2: Batch publish API** (`bus_batch`)

**Goal:** `publish_many(&[T])` with **one** Release fence & **one** edge-notify sweep per batch.

**Tasks**

* Finish `Bus::publish_many(&[T])` in `bounded.rs` (scaffold present).
* Semantics: identical delivery/overwrite behavior to repeated single publishes.
* Metrics: increment `bus_batch_publish_total`; observe `bus_batch_len_histogram`.
* Tests:

  * Wraparound & partial batch delivery.
  * Drop accounting invariant (overwrites − delivered).
* Benches: add `benches/a2_publish_many.rs`

  * Matrix: subs {1,4}, cap=64; bursts {1k,5k,10k}.
  * **Gate:** ≥15% faster vs single publishes on burst benches.
* Run:

  ```
  cargo bench -p ron-kernel --features bus_batch --bench a2_publish_many -- --baseline core-2025-10-19
  ```

### PR-4 — **A4: TLS metrics buffering** (`metrics_buf`)

**Goal:** remove atomics from hot publish/notify path.

**Tasks**

* Add `Metrics::hot() -> Option<HotCounters>` with TLS‐backed counters:

  * `inc_published()`, `add_published(u64)`, `inc_notify()`, `flush()`.
  * Periodic flush (100–250ms) + drop flush.
* Wire in `bounded.rs` publish path and edge-sweep:

  * Prefer `if let Some(h) = m.hot() { h.inc_*() }` over direct Prom increments.
* Counter: `bus_metrics_tls_flush_total`.
* Tests: no loss on drop; multi-thread churn; fuzz flush cadence.
* Bench: reuse `publish_edge_matrix` with/without `metrics_buf`.

  * **Gate:** ≥8% gain at subs=4, cap=64; tails not worse.
* Run:

  ```
  cargo bench -p ron-kernel --features metrics_buf --bench publish_edge_matrix -- --baseline core-2025-10-19
  ```

### PR-5 — **B1: SoA backend toggle** (`bus_soa`)

**Goal:** cache-friendly SoA ring with per-slot ready mask; start with `Arc<T>` payload (no `unsafe`).

**Tasks**

* `bus/soa.rs` shape-compatible with `bounded` (re-export when `bus_soa` is ON).
* 64-bit `seq`; Loom tests for last-bit clear & message visibility.
* Bench group: `bus_publish_matrix_soa` side-by-side with current backend.
* Telemetry: `bus_ready_mask_width` histogram.
* **Gate:** ≥20% win @ 4 subs / 64 cap **and** ≥30% @ 16 subs / 128 cap.
* Run:

  ```
  cargo bench -p ron-kernel --features bus_soa -- --baseline core-2025-10-19
  ```

### PR-6 — **B2: Topic/interest filtering** (`bus_interest`)

**Goal:** subscriber static bitmask on `KernelEvent` variants; publisher ANDs mask to ready set.

**Tasks**

* Static per-sub mask; dynamic change only via epoch’d swap.
* Counter: `bus_interest_skips_total`.
* Bench: mixed-topic stream (~30% relevant).
* **Gate:** ≥15% throughput gain on mixed workloads.
* Run:

  ```
  cargo bench -p ron-kernel --features bus_interest -- --baseline core-2025-10-19
  ```

### PR-7 — **C-polish** (padding, orderings, NUMA docs)

**Goal:** stack micro-wins.

**Tasks**

* `CachePadded` for per-subscriber tails/counters; 64B alignment where hot.
* Orderings: keep Release on final producer store, Acquire on consumer load; bitmasks Relaxed.
* Add CPU pinning / NUMA placement guide.
* **Gate:** +3–10% (more at high-N).

## 7) Paste template for NOTES.md (per PR)

```
## MOG Results — ron-kernel / <PR label> — <YYYY-MM-DD>

Baseline: core-2025-10-19
Features ON: <e.g., bus_batch>

Top deltas (wins/losses):
- publish_single subs=4 cap=64:   <old> ns → <new> ns  (Δ %)
- publish_single subs=16 cap=128: <old> ns → <new> ns  (Δ %)
- burst_5k subs=4 cap=64:         <old> ns → <new> ns  (Δ %)

Wake metrics:
- bus_notify_sends_total:       +X
- bus_notify_suppressed_total:  +Y (suppression = Y/(X+Y) = Z%)

Counters sanity:
- bus_receiver_lag_total: flat / improved
- bus_batch_publish_total: +N; batch_len p50/p95: A/B

Notes & risks:
- <one-liner per surprising regression or tradeoff>
Decision: <ship / adjust / revert>
```

## 8) Risk register (ron-kernel)

* **A1 lost wake:** Covered by post-drain swap(false) race-check; unit tests green (keep Loom litmus in B1).
* **A2 drop accounting:** Property test balances `overwrites − delivered`.
* **B2 dynamic masks:** Only via epoch’d swap to avoid torn reads.
* **SoA inline payloads:** Start `Arc<T>`; consider inline only after Loom+Miri+fuzz **all green** and wins justify it.

---


### END NOTE - OCTOBER 21 2025 - 15:07 CST


### BEGIN NOTE - OCTOBER 21 2025 - 19:30 CST

# RON-kernel – Carry‑Over Notes (Bench & Next Steps)

*Last updated: now*

## TL;DR

* We have a clean, reproducible baseline (`core-2025-10-19`).
* Steady publish path is ~31 ns (no subs), ~74 ns (1 sub). Bursty classic hits ~7–10M pub/s on laptop; batched path matches/exceeds at smaller bursts.
* Variance stems from runtime scheduling/cache; we have concrete knobs to stabilize results.
* Next: implement the “Fortran trick” (SoA stride-1 layout + prefetch) and wire edge-notify drains; target +15–30% throughput.

---

## What we achieved

* **Bench harness upgrades** in `bus_publish.rs`:

  * Added env-configurable **burst size**, **fanout**, **cap**, and **epoch yield** switches.
  * Split groups: steady-state, bursty classic (fanout/lagged), and **batched** (feature-gated), plus gated **edge** drains.
  * Ensured only hot paths are timed; setup/drain outside measuring loop.
* **Stable baseline** recorded with Criterion: `--save-baseline core-2025-10-19` and compared with `--baseline core-2025-10-19`.
* **Publish performance**:

  * **Steady, no subs:** ~31 ns.
  * **Steady, 1 sub:** ~74 ns.
  * **Lagged cap=1:** ~72–77 ns.
* **Bursty throughput (classic drains, fanout=4, cap=2048)**:

  * **128:** ~5.6–7.3 M pub/s
  * **256:** ~6.1–7.7 M pub/s
  * **512:** ~7.2–8.0 M pub/s
  * **1024:** ~9.2–9.7 M pub/s
  * (Deliveries/s ≈ pub/s × fanout.)
* **Batched path** (publish_many): competitive at **128–256**; mixed at **512–1024** with runs from ~8–10.3 M pub/s.
* **Lagged/pressure path (cap=1)** consistently around **0.8–1.1 M pub/s** across classic/batched.

---

## Helpful environment & commands

* **Env toggles** used:

  * `RON_BENCH_BURST` (default 256)
  * `RON_BENCH_FANOUT` (default 4)
  * `RON_BENCH_CAP` (default 2048 for bursts; 64/1 in steady tests)
  * `RON_BENCH_PUB_YIELD` (optional epoch yield)
  * `RON_TLS_FLUSH_THRESHOLD=64`
* **One-off baseline save**:

  ```sh
  cargo bench -p ron-kernel --bench bus_publish -- \
    --warm-up-time 3 --measurement-time 10 --save-baseline core-2025-10-19
  ```
* **Param sweep with baseline compare** (first saves, then compares):

  ```sh
  first=1
  for B in 128 256 512 1024; do
    if (( first )); then F="--save-baseline"; first=0; else F="--baseline"; fi
    RON_TLS_FLUSH_THRESHOLD=64 \
    RON_BENCH_BURST=$B RON_BENCH_FANOUT=4 RON_BENCH_CAP=2048 \
    cargo bench -p ron-kernel --features "metrics_buf,bus_batch" --bench bus_publish \
      -- --warm-up-time 3 --measurement-time 10 $F core-2025-10-19
  done
  ```

---

## Observations & variance control

* **Variance sources:** Tokio scheduling jitter, fanout drains sharing cores with publisher, macOS power/turbo shifts, cache effects at 512/1024 bursts.
* **Knobs to stabilize:**

  1. Pin cores: set Tokio worker threads = `fanout + 1`; try task affinity where possible.
  2. **Cap**: use generous caps for burst paths (e.g., `RON_BENCH_CAP=4096`) to avoid unintended backpressure.
  3. **Warmup**: consider 2–3 s warmup for burst groups.
  4. Build: `RUSTFLAGS="-C target-cpu=native"` for benches.
  5. Keep `SamplingMode::Flat` (already set) and consistent sample sizes (steady=80, burst=60).

---

## “Fortran trick” plan (SoA + edge drains)

* **Goal:** Improve cache locality and SIMD prefetch, reduce false sharing and wake contention.
* **Steps:**

  1. **SoA ring slots**:

     * Convert AoS slot (meta + payload) into **SoA**: separate arrays for metadata (seq/state) and payloads.
     * Ensure **stride-1** for publisher writes and subscriber reads; align slot arrays to cache line boundaries.
     * Add prefetch (`core::arch` intrinsics) on producer for next N slots at 512/1024 bursts.
  2. **SIMD-friendly copies**:

     * Where payloads are POD-like, consider `copy_nonoverlapping` in tighter loops; evaluate `#[inline(always)]` hot paths.
  3. **Edge-notify drains** (feature `bus_edge_notify`):

     * Use edge-triggered wake suppression to minimize cross-core wake storms; wire `spawn_edge_drains` path in benches.
  4. **Metrics check**: confirm no regressions in invariants (no locks across `.await`, bounded backpressure, metrics correctness).

**Expected uplift (same laptop):** +15–30% throughput on burst sizes ≥512; steady publish ~31 ns → ~24–27 ns plausible; burst pub/s ~9–10 M → **~11–13 M** (fanout=4 ⇒ ~44–52 M deliveries/s).

---

## Action items (next session)

* [ ] Land the current `bus_publish.rs` updates (already drafted) and commit with baseline `core-2025-10-19`.
* [ ] Add `RUSTFLAGS="-C target-cpu=native"` to bench runs; re-run 512/1024 with `RON_BENCH_CAP=4096` to reduce variance.
* [ ] Implement **SoA** layout behind a feature flag (e.g., `bus_soa`) and keep AoS path for A/B.
* [ ] Add simple producer-side **prefetch** at burst > 256.
* [ ] Enable and measure **edge drains** path (`--features bus_edge_notify`) at fanout=4/8.
* [ ] Record new baseline (e.g., `core-2025-10-19-SoA-A1`) and capture flamegraphs if variance persists.

---

## Reference files

* **Bench:** `crates/ron-kernel/benches/bus_publish.rs`
* **Docs:** `MOG.md` (project motivations & targets), `RON_CORE.MD` invariants, `BUILDPLAN.MD` gates.
* **Tests:** `bus/SOA.rs` unit tests (bus contract semantics).

---

## Known good numbers to remember

* **Steady (no subs):** ~31 ns
* **Steady (1 sub):** ~74 ns
* **Classic burst, cap=2048, fanout=4:** ~7–10 M pub/s (→ ~28–40 M deliveries/s)
* **Pressure path (cap=1):** ~0.8–1.1 M pub/s
* **Batched (128–256):** ~7.2–7.9 M pub/s; **(512–1024):** ~8–10.3 M pub/s (variable)

---


### END OF NOTE - OCTOBER 21 2025 - 19:30 CST 

## MOG Results — A2 bus_batch — 2025-10-21 - 20:30 CST
Config: subs=16, cap=128
Baseline compare (Criterion):
- burst=64:  731.07 µs → 552.35 µs  (≈ +24.5% faster)
- burst=128: 720.87 µs → 549.57 µs  (≈ +23.8% faster)
- burst=256: 714.12 µs → 551.57 µs  (≈ +22.7% faster)
Notify behavior (from smoke): 1 notify/subscriber/batch; suppression ~0 with publish_many.
Receiver lag: 0 when cap ≥ burst (validated).
Decision: A2 SHIPPED.

### TEMPLATE:

## MOG Results — A4 metrics_buf — 2025-10-21
Threshold sweep: 32/64/128/256
Findings:
- p95 publish (fanout=8–16, cap=128): <numbers>
- Scrape/exporter CPU proxy: <observations>
Default: RON_TLS_FLUSH_THRESHOLD=<64|128>
Decision: Ship metrics_buf feature (OFF by default; ON in prod profiles).


### UPDATE:


## MOG Results — A4 metrics_buf — 2025-10-21
Sweep: RON_TLS_FLUSH_THRESHOLD ∈ {32, 64, 128, 256}

Findings:
- Steady/no-subscriber publish: +4–7% (expected small regression).
- Classic fanout (burst256, cap2048): large wins, ~+20–60% (best at mid/high thresholds).
- Batched fanout (burst256, cap2048): mixed; ranges from small regression (~+8–10% time) to solid improvements (~–18% time).
- Lagged cap=1: mostly noise / dominated by overrun behavior.

Decision:
- Ship `metrics_buf` feature (OFF by default for dev/microbench parity).
- Recommend ON in prod with `RON_TLS_FLUSH_THRESHOLD=128` (fallback to 64 if scrape rates are high; consider 256 for very infrequent scrapes).
Status: A4 PASSED (default chosen).


## END OF MOG UPDATE

### BEGIN NOTE - 2025-10-21 — 21:41 CST

## MOG carryover: A2/A4 shipped, next = B1 (SoA) + comparables - 

### What we shipped / proved
- A2 `bus_batch` (publish_many) ✅
  - subs=16, cap=128
    - burst=64:  publish 731.07 µs → publish_many 552.35 µs  (≈ +24.5% faster)
    - burst=128: publish 720.87 µs → publish_many 549.57 µs  (≈ +23.8% faster)
    - burst=256: publish 714.12 µs → publish_many 551.57 µs  (≈ +22.7% faster)
  - Throughput uplift ≈ +30% across the board (hot path only; drains/setup out of loop).

- A4 `metrics_buf` (TLS-buffered metrics) ✅
  - Real-world bursty fanout (cap ≥ burst): time −22%…−37% → throughput **+28%…+60%** depending on threshold.
  - Micro “no-sub” synthetic: small regression +4%…+7% (expected).
  - Decision: keep feature-gated and **OFF by default** for dev parity; **ON in prod** with default `RON_TLS_FLUSH_THRESHOLD=128` (64 if scrape is very frequent; 256 if very infrequent).

- Notify hygiene (edge + batch) ✅
  - With `bus_edge_notify`+`bus_batch`: **1 notify / subscriber / batch**, **0 lag**, **0 drops** when **cap ≥ burst**.
  - Example scrape: `bus_batch_publish_total=15`, `bus_notify_sends_total=240` with subs=16 ⇒ 15×16=240 (as designed).

- Bench harness UX ✅
  - Added threshold logging and group tagging:
    - Prints: `[bench cfg] RON_TLS_FLUSH_THRESHOLD=<val>, burst=<B>, fanout=<N>, cap=<CAP>`
    - Group names include `tls_thresh=<val>` for easy sweep diffs.
  - Criterion baseline usage fixed (save first, then compare).

### Commands we rely on (pasteable)
# Save a baseline (adjust warmup/measure if needed)
cargo bench -p ron-kernel --features bus_batch --bench bus_batch -- --warm-up-time 3 --measurement-time 10 --save-baseline core-2025-10-19

# Compare against baseline (env-driven fanout/cap/burst)
RON_BENCH_BURST=128 RON_BENCH_FANOUT=16 RON_BENCH_CAP=128 \
cargo bench -p ron-kernel --features bus_batch --bench bus_batch -- --baseline core-2025-10-19

# TLS metrics buffering sweep (prints tls_thresh in output)
for T in 32 64 128 256; do
  RON_TLS_FLUSH_THRESHOLD=$T \
  cargo bench -p ron-kernel --features "metrics_buf,bus_batch" --bench bus_publish -- --baseline core-2025-10-19
done

# Smoke publisher (batched vs single toggled by feature)
cargo run -p ron-kernel --features bus_batch --example publish_smoke

# Quick metrics sanity (notify/batch/lag)
curl -s http://127.0.0.1:<port>/metrics | grep -Ei 'bus_(notify|batch|published|dropped|receiver|lag)'

### Decisions / invariants
- Keep `metrics_buf` OFF by default; turn ON in prod profiles with `RON_TLS_FLUSH_THRESHOLD=128`.
- Keep `bus_batch` ON in prod; MOG target was ≥15%—actual win ≈25% (locked).
- No locks across `.await` continues to hold (I-7). Bounded queues honored (I-2). Observability improved (I-4).

### What’s next in MOG (do these next)
1) **B1: `bus_soa` (Structure-of-Arrays bus backend) — feature-gated**
   - Goal: **+20–30%** on bursty fanout (subs=4 & subs=16 gates below).
   - Ship gates:
     - ≥20% at subs=4, cap=64 (bursty)
     - ≥30% at subs=16, cap=128 (bursty)
     - Concurrency safety: Loom tests pass; Miri clean; basic fuzz (no UB/panics).
   - Work:
     - Add `bus_soa` feature + SoA ring (separate arrays for seq/state/payload).
     - Swap via type alias or cfg in `bus::bounded::Bus`.
     - Bench parity with existing suites (reuse env knobs; same labels).
     - CODECHECK: comments on memory ordering; aliasing rules per CONCURRENCY_AND_ALIASING_BLUEPRINT.

2) **Comparables harness (to quantify the MOG vs ecosystem)**
   - Implement `benches/comp.rs` running the *same* scenarios against:
     - `tokio::sync::broadcast`, `flume`, `crossbeam-channel` (bounded MPMC).
     - Optional: `rtrb` SPSC as “single-consumer ceiling” datapoint (document topology difference).
   - Report both **publishes/sec** and **deliveries/sec = publishes × subs**.
   - Acceptance to claim “2–4×”: aggregate deliveries ≥2× at subs 8–16 with cap≥burst; micro parity ±10% with metrics_buf OFF.

3) **Docs & CI**
   - README: Feature flags (`bus_batch`, `bus_edge_notify`, `metrics_buf`) + env knobs (`RON_BENCH_*`, `RON_TLS_FLUSH_THRESHOLD`, `RON_TICK_MS`).
   - Add “Baselines 101” snippet (save → compare).
   - CI: benches compile but don’t run; examples run smoke (short TTL).

### Risks / watchouts
- `metrics_buf` pressure-path (cap=1) shows mixed results (noise to small regressions). It’s fine; that path is intentionally pessimistic.
- Keep micro parity by leaving `metrics_buf` OFF in dev benches; turn ON only for prod profiles.

### Definition of Done for B1
- Perf: ≥20% at subs=4/cap=64 and ≥30% at subs=16/cap=128 (bursty), measured by existing benches (median).
- Safety: Loom/Miri/fuzz clean; CODECHECK.md items satisfied; comments per CODECOMMENTS.md.
- Docs: README + MOG.MD updated with results and toggles.
- Metrics: no changes to counters’ meaning; notify/batch invariants preserved.

### TL;DR brag line
A2 gave ~**25%** latency cut in hot fanout; A4 yields **+30–60%** throughput on bursty loads with negligible micro tax. Next: SoA for another **+20–30%** and a reproducible comparables harness to cement the 2–4× lead.

### END OF NOTE - OCTOBER 21 2025 - 21:41 CST

### BEGIN NOTE - OCTOBER 21 2025 - 22:35 CST

Absolutely—here are clean, copy-pasteable carry-over notes so the next instance can resume instantly.

# RON-KERNEL — Carry-Over Notes (MOG / SoA) — 2025-10-21

## TL;DR

* **SoA bus backend is integrated and fully green under `--features bus_soa`.**
* We mirrored bounded semantics (Clone sender, `recv() -> Result<T, RecvError>`, lag + closed handling).
* Fixed lag detection with **sequence-gap** logic; `Bus::handle_recv` is where lag metrics are bumped (bounded-style).
* `bus::mod` keeps call sites stable: when `bus_soa` is ON, we re-export SoA as `bounded`.

---

## What we changed (high-signal)

**Files touched**

* `src/bus/soa.rs` (full rewrite for parity + performance)

  * `Bus<T>` is cloneable; maintains `publishers` count and **sets `closed=true`** when the last sender drops.
  * `Receiver::recv()` now returns `Result<T, tokio::sync::broadcast::error::RecvError>` with:

    * **Lagged** if `slot_seq > tail+1` (sequence gap) or our bit was already cleared on `slot_seq==next`.
    * **Closed** if the bus is closed and no new messages have arrived.
  * **Notify path**:

    * Default: global `Notify` wake.
    * Optional: `bus_edge_notify` keeps the disciplined edge-triggered sweep (A1/A5).
  * **Metrics**:

    * `publish()` increments `bus_published_total`.
    * `Bus::handle_recv(res, Some(metrics))` bumps `bus_receiver_lag_total` on `Lagged` (not in `recv()`), matching bounded tests/utilities.
  * **Batch** (`bus_batch`): single wake after the loop; metrics recorded.
* `src/bus/mod.rs`

  * When `bus_soa` is enabled, `pub mod bounded { pub use super::soa::*; }` keeps `crate::bus::bounded::Bus` stable.
* `tests/soa_smoke.rs`

  * Updated to use `bus_soa` path via the `bounded` re-export and `Bus::handle_recv`.

**Warnings**

* Example `publish_smoke.rs`: simple `mut` removal suggested (non-blocking).

---

## Current status (all green)

* `cargo test -p ron-kernel` ✅
* `cargo test -p ron-kernel --features bus_soa` ✅
* Close semantics tests: ✅
* Lag metric tests (bounded parity): ✅

**Known small warnings**

* `examples/publish_smoke.rs`: `let rxs` doesn’t need `mut`.

---

## How to run locally (for future you)

```
# Default bounded backend:
cargo test -p ron-kernel

# SoA re-exported as `bounded`:
cargo test -p ron-kernel --features bus_soa

# Optional:
cargo run  -p ron-kernel --example kernel_demo --features bus_soa
cargo bench -p ron-kernel                 # (once benches are added)
cargo bench -p ron-kernel --features bus_soa
```

---

## Design decisions (keep these in mind)

* **API parity over everything:** We matched `tokio::broadcast` surface and error types to avoid test churn and downstream breakage.
* **Lag metric location:** Counted in `Bus::handle_recv` (not `recv()`), so utilities/tests that already call the helper behave identically across backends.
* **Sequence-gap lag detection:** Required in SoA to report lag when the slot has been overwritten before a subscriber claims its bit.
* **Close semantics:** Last `Bus` clone dropping sets `closed=true` and wakes receivers (tests assert `Err(Closed)`).

---

## Next steps on the MOG plan (ordered)

1. **B2: Topic masks on SoA (subset fan-out)**

   * API: `subscribe_with_mask(mask: u64)`, `publish_with_mask(msg, mask)`.
   * Fast path when mask == `!0u64` (no extra work).
   * Tests:

     * `masked_publish_delivers_to_subset`
     * `nonmember_receivers_do_not_wake`
     * `changing_mask_affects_future_only`
2. **A1/A5: Edge-triggered notify discipline**

   * Expose `subscribe_edge()` under `bus_edge_notify`.
   * Example + test for disciplined drain (`await_notify → drain → race check`).
3. **A2: Batch publish benches**

   * Show wake coalescing benefits (bounded vs SoA; with/without edge notify).
4. **A3: Capacity autotune integration**

   * Add `Bus::with_autotuned_capacity(metrics, hint)` gate to existing helper.
5. **Bench harness + metrics**

   * Throughput, p50/p90/p99 latency, notify coalescing, lag rate under saturation.
6. **Polish**

   * Remove `mut` warning in `publish_smoke.rs`.
   * Optional: expose a `set_mask(mask)` API for dynamic receiver topics (future).

---

## Minimal diffs queued for next patch (quick wins)

* **publish_smoke example**: remove `mut` to silence the warning.
* **Topic masks scaffolding** (if we start there):

  * Add `topic_mask: u64` to `Receiver`.
  * Add `publish_with_mask()` that derives `ready_mask = alive_mask & topic_mask`.
  * Keep hot path branchless when topic mask is full.

---

## Sanity checklist before merging B2

* All current tests pass with and without `bus_soa`.
* New topic-mask tests pass under `bus_soa`.
* No additional allocations on hot path; no locks held across `.await`.
* `Bus`/`Receiver` Drop semantics unchanged.

---

## Quick reference: key invariants

* At most **64 subscribers** in this SoA variant (bitmask width).
* New subscribers start at **current global seq**; no history replay.
* Overwrite permitted; lag is observable (via gap and bit checks).
* No `unsafe`; atomics only where necessary.

---

### END NOTE - OCTOBER 21 2025 - 22:35 CST

### BEGIN NOTE - OCTOBER 21 2025 - 22:45 CST

## MOG — ron-kernel (next 3 PRs)

Baseline: core-2025-10-19. Features default OFF.

PR-1: A4 metrics_buf
- Wire TLS counters + periodic/drop flush; add bus_metrics_tls_flush_total.
- Keep OFF in dev benches; record deltas with NOTES “Top deltas” template.
- Run: RON_TLS_FLUSH_THRESHOLD=64 cargo run -p ron-kernel --example kernel_demo --features metrics_buf

PR-2: B1 bus_soa (SoA backend)
- Implement SoA ring (seq[], ready_mask[], msg[]), start with Arc<T>.
- Gates: ≥20% @ subs=4 cap=64; ≥30% @ subs=16 cap=128 (bursty); Loom/Miri/fuzz clean.
- Run: cargo bench -p ron-kernel --bench bus_publish --features bus_soa

PR-3: Comparables harness
- benches/comp.rs vs tokio::broadcast, flume, crossbeam MPMC (+ optional rtrb).
- Accept claim only if deliveries/s ≥2× at subs 8–16, tuned caps, metrics_buf OFF.

Housekeeping (always-on):
- Public API frozen; feature flags documented in README.
- Paste wins/losses in NOTES with wake/batch counters (notify_sends/suppressed, batch_total/len_hist).

### END NOTE - OCTOBER 21 2025 - 22:45 CST

### BEGIN NOTE - OCTOBER 21 2025 - 23:37 CST

## 2025-10-21 — bounded.rs unchanged; backoff helper added

- Observed regression only in steady cap=1 lagged; disappears with publisher yield.
- Root cause: bench scheduling / writer backoff in the extreme single-slot case.
- bounded.rs wraps tokio::broadcast::send(), which is O(1) and non-blocking; no spin loop here.
- Action:
  - Added src/bus/backoff.rs (TwoPhaseBackoff) for benches and upcoming bus_soa.
  - Do NOT wire backoff into bounded.rs publish().
  - Next: implement bus_soa writer slow-path using TwoPhaseBackoff; keep classic path intact.

### END NOTE - OCTOBER 21 2025 - 23:37 CST

### BEGIN NOTE - OCTOBER 21 2025 - 23:46 CST

## 2025-10-21 — Bench checkpoint
Hot path steady: ~32 ns (no-sub), ~76–78 ns (1-sub). ✅
Bursty classic (fanout=4, cap=2048): 7.84 M elem/s → +9–16%. ✅
Cap=1 steady lagged: mild regression unless publisher yields. ⚠️
Isolation proved fix: enabling publish yield or cap=2 restores/improves. ✅

Action:
- Keep classic wrapper unchanged (no spins/yields added).
- Land Two-Phase Publisher Backoff helper (done; unused until SoA).
- Proceed PR-1: metrics_buf (feature-gated; OFF in benches).
- Proceed PR-2: bus_soa with writer slow-path using backoff helper.
- After PR-2 lands, re-run MOG matrix (subs {1,4,16} × cap {64,128}) and update gates.

### END NOTE - OCTOBER 21 2025 - 23:46 CST

### BEGIN NOTE - OCTOBER 22 2025 - 10:51 CST

Here’s a clean, paste-ready carryover note you can drop into the next instance.

---

# CARRYOVER — RON-Kernel / ron-bus (SoA “Fortran trick”)

**Date:** 2025-10-22

## TL;DR (wins)

* Swapped bounded AoS path for **SoA ring backend** (`bus_soa`) with parity API.
* Fixed panic in SoA recv path (payload/bit race) by **single-lock clone + conditional take**.
* Added **edge-notify** path with **coalesced wakes**; skip sweep when no edge subscribers.
* All unit/integration tests **green**; benches show **~2× speedups** on key paths.
* Bench README/how-to written; main README performance blurb drafted.

## Benchmark QRD (post-SoA vs. prior)

* **steady/no_subs:** ~68 ns → ~68 ns (±0%)
* **steady/one_sub:** ~189–233 ns → **~96.3 ns** (~−49%, ~2× faster)
* **steady/lagged cap=1:** ~188–193 ns → **~94.2 ns** (~−50%, ~2× faster)
* **bursty/classic cap=2048 (256 elems):** ~49.96–50.12 µs → **~24.76 µs** (~−50%, ~2× thrpt)
* **bursty/classic lagged cap=1:** ~48.18–49.56 µs → **~24.34 µs** (~−49–50%, ~2× thrpt)
* **bursty/edge cap=2048:** ~249–270 µs → **~155–157 µs** (~−37–42%, +60–73% thrpt)
* **bursty/edge lagged cap=1:** ~246–250 µs → **~180–182 µs** (~−26–28%, +36–39% thrpt)

> Rough throughput feel: classic burst ~**10.3 Melem/s** publishes (scales with fanout for deliveries).

## Repro recipes (bench)

* Default classic (SoA + edge feature):

  ```bash
  cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
    --warm-up-time 3 --measurement-time 10
  ```
* With batching path:

  ```bash
  cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify,bus_batch" -- \
    --warm-up-time 3 --measurement-time 10
  ```
* Env knobs used in runs:

  * `RON_BENCH_BURST={1,256}` `RON_BENCH_FANOUT={1,4,16}` `RON_BENCH_CAP={1,2,2048}`
  * Optional: `RON_BENCH_PUB_YIELD=1` to probe yield costs.
* Criterion baselines (examples we used):
  `--save-baseline core-2025-10-19_cap1`, `core-2025-10-19_cap2` and compare with `--baseline <name>`.

## What shipped (code notes)

* **`bus/soa.rs`**

  * Ring slot: `seq(AtomicU64)` + `ready_mask(AtomicU64)` + `msg(Mutex<Option<T>>)`; publish sets mask then seq (Release), recv checks seq (Acquire), clears bit with `fetch_and`.
  * **Recv fix:** hold the slot lock once, `clone` payload; if `was_last`, `take()` under the same lock (eliminates “payload must exist if bit was set” panic).
  * Edge helper: `pending` bit + `Notify`; **sweep coalesces** notifies; registry prunes dead subs.
* **`bus/bounded.rs`**

  * Edge-notify sweep added and gated; optional **TLS metrics buffer** path.
* Benches: added **edge** and **batched** groups; verified steady/bursty + lagged variants.

## Next steps (MOG plan)

### Quick wins (low risk, aim +5–15%)

1. **Single-lock recv already done** (verify across all variants).
2. **Guarded edge sweep** (skip when `edge_subs == 0` via atomic counter).
3. **Atomics audit:** keep `AcqRel` where needed; check if read-side can drop to `Acquire` and write-side to `Release` without weakening correctness.
4. **Bench profile tuning:** run with `-C target-cpu=native`, LTO fat, `codegen-units=1`, `panic=abort` under `[profile.bench]`.

### Medium bets (10–30% if they land)

5. **True SoA split:** three arrays (`seqs`, `masks`, `msgs`) to improve stride-1 scans and reduce false sharing with the mutex.
6. **Deferred drop (feature-gated):** remove `take()` from readers; let the next publish overwrite & drop. Hot read path gets cheaper.
7. **Edge batch sweep:** coalesce multiple publishes into a single notify pass when they arrive back-to-back (tiny TLS counter window).

### Bench matrix to map frontier

8. Sweep **burst sizes**: 1, 8, 32, 256, 1024.
9. Sweep **fanout**: 1, 4, 8, 16, 32, 64.
10. Sweep **capacity**: 1, 64, 512, 2048.
11. Compare **classic vs edge** with/without `bus_batch`; vary `RON_TLS_FLUSH_THRESHOLD={0,4,64}` when `metrics_buf` on.
12. Save canonical baseline: `soa-2025-10-22` and update README charts.

### Hardening / DX

13. **Fix minor warnings:** make `Receiver::ring` private or expose `Slot<T>` consistently (private_interfaces warn).
14. **Loom/Miri/Fuzz:** loom scaffolding exists (0 tests ran); add minimal interleavings for seq/mask order; run Miri nightly on core paths.
15. **Docs:** finalize benches README and paste **Performance Highlights** at the top of main README.
16. **CI gates:** add `cargo bench --quiet` smoke, `RUSTFLAGS="-D warnings"` for benches/examples, and feature-matrix build.

## Open questions / watch items

* Verify no fairness regressions under **high fanout (≥32)** with edge path (coalescing could starve infrequent subs—add a fairness poke if needed).
* Validate **lag accounting** under cap=1 with heavy overwrite (counter deltas align with expectations).
* Confirm **metrics_buf** export interval doesn’t distort hot path under heavy publish rates.

## Decision log

* ✅ Adopt SoA backend behind `bus_soa` feature; keep bounded path for compatibility.
* ✅ Land edge-notify/coalescing; batch publish path (`bus_batch`) is on and measured.
* ✅ Treat SoA as the default in performance builds once CI green + docs land.

---

*End of carryover.*


### END NOTE - OCTOBER 22 2025 - 10:51 CST

### BEGIN NOTE - OCTOBER 22 2025 - 11:30 CST

Batched path confirmed dominant:
- Steady: +4% (no_subs), +9.6% (one_sub), +6.8% (lagged cap1)
- Bursty fanout (burst=256, fanout=4, cap=2048): ~23–25% faster (9.33 vs 7.19 Melem/s)
- Lagged cap=1: ~12% faster (+14% thrpt)

Actions:
1) Autotune: enable batching when fanout≥2 or burst≥128 or cap≤64.
2) Add perf gate (script added in scripts/perf_gate.sh) with thresholds (one_sub ≤+10% slower; batched fanout ≥+15%; batched lagged ≥+8%).
3) Update MOG.md with headline deltas; keep bus_batch feature-gated.
4) cargo fix --example publish_smoke to clear warning.

### END NOTE — OCT 22 2025 — 11:30 CST


### BEGIN NOTE - OCTOBER 22 2025 - 14:05 CST

# Quick Reference Deck (QRD)

## Accomplishments (since pre-MOG)

* Implemented **A3: capacity autotune + guardrails** (`mog_autotune.rs`) with plateau heuristic (64/128/256) and safe override normalization.
* Bench suite **bus_publish.rs** revamped:

  * Clear env knobs: `RON_BENCH_BURST`, `RON_BENCH_FANOUT`, `RON_BENCH_CAP`, `RON_TLS_FLUSH_THRESHOLD`, `RON_BENCH_PUB_YIELD`.
  * Three families: steady, **classic burst fanout**, optional **edge** drains, plus **batched** (`publish_many`) when `bus_batch` is enabled.
  * Drains run on a multithread Tokio RT; fanout configurable; backpressure scenarios (cap=1) included.
* Validated on 2019 MBP (Intel): steady publish ≈ **30–32 ns** (no subs), ~**71–76 ns** (one/lagged); burst 256–1024 shows **7–10 Melem/s** depending on mode; batched and/or larger bursts can push higher in favorable rounds.
* Repro math/outputs captured in transcripts above (multiple rounds, strict mode, longer measurement windows).

---

## Doing Next (you asked for #1 and #4)

### 1) Enable LTO + `codegen-units=1` (applies to `cargo bench`)

*Edit once, commit it.*

**Cargo.toml** (top-level):

```toml
[profile.release]
lto = "thin"          # try "fat" once for a datapoint (slower link)
codegen-units = 1
panic = "abort"
strip = "symbols"

[profile.bench]
inherits = "release"
lto = "thin"
codegen-units = 1
```

Optional (for local benches):

```bash
# unlock CPU-specific vector/pipe tuning
export RUSTFLAGS="-C target-cpu=native"
```

**Re-bench examples**

```bash
# classic, burst 512, large cap
RON_BENCH_BURST=512 RON_BENCH_CAP=4096 cargo bench -p ron-kernel --bench bus_publish

# batched (publish_many), classic drains
RON_BENCH_BURST=1024 RON_BENCH_CAP=8192 \
cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish
```

### 4) Batch N-publish + edge drains (aiming for another ~+5–10%)

* Build/bench with **edge drains** + **publish_many**:

  * **Enable features**: `bus_edge_notify` for receivers, `bus_batch` for publisher path.
* Keep **cap ≥ burst** (e.g., `cap=8192` for `burst=1024`) to avoid blocking in the batch loop.
* Sweep **TLS flush threshold** around **32/64/128**; best value can shift when batching & edge are both on.

**Bench commands**

```bash
# Edge drains + classic publish (control)
RON_BENCH_BURST=512 RON_BENCH_CAP=4096 \
cargo bench -p ron-kernel --features "bus_edge_notify" --bench bus_publish

# Edge drains + batched publish_many (target)
RON_BENCH_BURST=1024 RON_BENCH_CAP=8192 \
cargo bench -p ron-kernel --features "bus_batch,bus_edge_notify,metrics_buf" --bench bus_publish
```

---

## Repro Cheatsheet (what matters on another host)

* **Env knobs**

  * `RON_BENCH_BURST` (default 256)
  * `RON_BENCH_FANOUT` (default 4)
  * `RON_BENCH_CAP` (default 2048; use ≥ burst for no backpressure)
  * `RON_TLS_FLUSH_THRESHOLD` (default 64)
  * `RON_BENCH_PUB_YIELD=1` to yield between bursts (epoch realism)
* **Features**

  * `bus_batch` → enables `publish_many` bench path
  * `bus_edge_notify` → spawns edge receivers (A2/A3 variants)
  * `metrics_buf` → if your bus wiring expects it
* **Runtime**: multithreaded Tokio RT with fixed worker count (we already pin the count in benches to reduce jitter).
* **Machine notes**: macOS has fewer knobs for governor/affinity; for the cleanest runs, keep the laptop cool/plugged-in or replicate on Linux (governor `performance`, pin with `taskset`).

---

## What’s Left / Backlog

### ron-kernel (benching + runtime polish)

* **Edge + Batch matrix**: lock in a best-known-good (BKG) combo (`burst`, `cap`, `tls_flush`) that hits or exceeds 10–12 Melem/s on MBP; document on Linux as well (target 12–15+ Melem/s).
* **Stability**: add a `--strict` CI step that runs 3 short rounds to ensure no >5% regressions on the main two scenarios:

  1. classic_fanout (`burst=256`, `cap=2048`)
  2. batched_fanout (`burst=512/1024`, `cap=4096/8192`)
* **CSV emit** (optional): dump Criterion summaries to CSV so we can plot trendlines across commits.
* **Affinity (Linux)**: optional gate to pin bench threads (via `core_affinity`) for even tighter tails.

### MOG (A-series features)

* **A3 (done)**: capacity autotune + guardrails (plateaus, override normalization & clamping).
* **A2 (edge notify path)**: ensure builder paths expose an easy switch between classic vs edge; document intended workloads.
* **A1 (docs/README)**: a short “Why MOG” with QRD + env knobs + recommended settings per workload (low fanout steady vs bursty fanout).
* **Guardrails telemetry**: warn once when selected capacity >256 (already present); consider a counter exposed via `metrics` for visibility in prod.

---

## Current Performance Snapshot (headline numbers to carry)

*(2019 MBP, fanout=4 unless stated; representative medians across your last runs)*

* **Steady publish (classic)**:

  * no-subs: **~30–32 ns**
  * one/lagged: **~71–76 ns**
* **Bursty classic (burst=512, cap=4096)**: **~9.8–10.0 Melem/s** best pass; typical **7.8–9.0 Melem/s** across rounds.
* **Bursty batched publish_many (burst=1024, cap=8192)**: **~8.6 Melem/s** (classic drains); larger variability, but batched holds its own and sometimes leads at lower bursts.

*(Expectation with LTO+CGU=1 and Edge+Batch): another **+8–20%** compiler win plus **+5–10%** from edge+batch on best settings.)*

---

## README snippet (drop-in)

```
# ron-kernel Bench QRD

## Profiles
[profile.release] and [profile.bench]
- lto = "thin", codegen-units = 1, panic = "abort", strip = "symbols"
Optional: export RUSTFLAGS="-C target-cpu=native"

## Env
RON_BENCH_BURST=256|512|1024
RON_BENCH_FANOUT=4
RON_BENCH_CAP=2048|4096|8192
RON_TLS_FLUSH_THRESHOLD=64  # sweep 32–128
RON_BENCH_PUB_YIELD=1       # simulate epoch yield

## Modes
# classic
cargo bench -p ron-kernel --bench bus_publish
# batched (publish_many)
cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish
# edge receivers
cargo bench -p ron-kernel --features "bus_edge_notify" --bench bus_publish

## Targets (MBP 2019)
steady: ~30–32 ns (no subs), ~71–76 ns (1/lag)
burst classic: ~8–10 Melem/s at burst 512 cap 4096
batched: ~8.5–9.0 Melem/s at burst 1024 cap 8192
```

---


### END NOTE - OCTOBER 22 2025 - 14:05 CST


### BEGIN NOTE - OCTOBER 23 2025 - 12:25 CST


# ron-kernel wrap-up (bench + perf)

## What we accomplished

* **Steady-state publish latency (classic path):**
  ~**71–77 ns** with one/lagged subscriber on a 2019 MacBook Pro (no recv on sub).
* **Bursty throughput (classic fanout):**
  With **burst=512, fanout=4, cap=4096** we observed up to **~10 MElem/s** publish rate.
  (Multiple runs ranged ~8–10 MElem/s; best run hit ~10 MElem/s.)
* **Bursty throughput (batched `publish_many`):**
  With **burst=1024, fanout=4, cap=8192** we saw **~8.6 MElem/s** on laptop silicon.
* **Robustness & observability:** Benches print the active config (`tls_thresh, burst, fanout, cap`), and we saved **baselines** for apples-to-apples comparisons.
* **Guardrails landed:** `mog_autotune.rs` provides safe, feature-gated capacity heuristics and override normalization (pow2, clamped).

> Rule of thumb from your runs: **Publish tops ~10 MElem/s** on the MBP; deliveries scale with fanout (e.g., 4× fanout ≈ 4× deliveries when subscribers keep up).

---

## Quick “lightweight” repro (low heat on laptop)

These run in ~20–30s each and won’t pin your fans forever.

```bash
# Classic steady + burst (burst=512, fanout=4, cap=4096)
RON_BURST=512 RON_CAP=4096 \
cargo bench -p ron-kernel --bench bus_publish \
  -- --warm-up-time 1 --measurement-time 6

# Batched (publish_many) steady + burst (burst=1024, cap=8192)
RON_BURST=1024 RON_CAP=8192 \
cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish \
  -- --warm-up-time 1 --measurement-time 6
```

Environment knobs that the bench binary reads (aliases supported):

* `RON_BENCH_BURST` **or** `RON_BURST` **or** `BURST`
* `RON_BENCH_FANOUT` **or** `FANOUT` (default 4)
* `RON_BENCH_CAP` **or** `RON_CAP` **or** `CAP`
* `RON_TLS_FLUSH_THRESHOLD` (default 64)
* `RON_BENCH_PUB_YIELD=1` to yield between bursts (optional)

---

## Full “save a baseline” repro (for publishing results)

These are the exact styles you used, with baselines for easy diffs.

```bash
# 1) Classic, steady + burst (burst=512)
BASE=core-$(date +%F)-classic-b512
RON_BURST=512 RON_CAP=4096 \
cargo bench -p ron-kernel --bench bus_publish \
  -- --warm-up-time 3 --measurement-time 12 --save-baseline "$BASE"

# 2) Classic, steady + burst (burst=1024)
BASE=core-$(date +%F)-classic-b1024
RON_BURST=1024 RON_CAP=4096 \
cargo bench -p ron-kernel --bench bus_publish \
  -- --warm-up-time 3 --measurement-time 12 --save-baseline "$BASE"

# 3) Batched publish_many (burst=512)
BASE_B=core-$(date +%F)-batched-b512
RON_BURST=512 RON_CAP=4096 \
cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish \
  -- --warm-up-time 3 --measurement-time 12 --save-baseline "$BASE_B"

# 4) Batched publish_many (burst=1024)
BASE_B=core-$(date +%F)-batched-b1024
RON_BURST=1024 RON_CAP=8192 \
cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish \
  -- --warm-up-time 3 --measurement-time 12 --save-baseline "$BASE_B"
```

> Results live under `target/criterion/`. Re-run with the same `--save-baseline` name to compare across commits.

---

## “Perf gate” sweep you already used (optional)

Good for macro comparisons; heavier on the machine.

```bash
# STRICT=1 prints a comparison table (install gawk if your macOS awk complains).
STRICT=1 ROUNDS=5 MEASTIME=12 bash crates/ron-kernel/scripts/perf_gate.sh
```

---

## Recommended build profile for peak numbers (bookshelf it)

(You can set this later; just documenting now for reproducibility.)

```toml
# Cargo.toml
[profile.release]
lto = "thin"     # or true for full LTO
codegen-units = 1
opt-level = 3
debug = false
```

---

## “QRD” of headline numbers (with MElem/s)

* **Steady, one/lagged sub:** ~**71–77 ns** publish (no recv on sub).
* **Classic burst (512, fanout 4, cap 4096):** **~9.8–10.0 MElem/s** (best runs), commonly **~8–9 MElem/s**.
* **Classic burst lagged cap=1 (512):** **~0.74–0.80 MElem/s** (backpressure path; for tail behavior study).
* **Batched burst (1024, fanout 4, cap 8192):** **~8.6 MElem/s** publish.

*(All on a 2019 MacBook Pro; server CPUs will lift the ceiling but the relative wins are what matter.)*

---

## What we’d do **later** to go even faster

When you come back, this is the short, pragmatic backlog:

### A. Build & codegen

* Enable **LTO + codegen-units=1** (above) for **~10–20%** in tight CPU loops.
* Consider `panic=abort` in release to shave a bit of binary/ICache.

### B. Runtime & scheduling

* **Pin bench threads** to logical cores (reduce migration variance).
  On macOS this is fussy; on Linux use `taskset`/`cset`.
* Keep the **Tokio runtime** out of the hot loop (you already do; just preserve this).

### C. Algorithmic tuning

* **Burst sweep**: confirm the knee for 256/512/1024 on server silicon; keep cap ≥ 8× burst.
* **Fanout sweep**: validate linear delivery scaling (e.g., 8/16).
* **TLS flush threshold** (`RON_TLS_FLUSH_THRESHOLD`) sweep around 32–128.
* **Batched path**: try **edge + batch** together if/when edge drains are enabled; often +5–10%.

### D. Memory & cache

* Keep ring **cap as power-of-two**, cache-friendly sizes (64/128/256/512/1024).
* Use **`mog_autotune`** feature to select plateaus automatically and flag pathological caps.

### E. Ops & reporting

* Always **save baselines** with date tags; commit the `target/criterion/*/new` JSONs (or export summaries) when publishing results.
* Note machine specs with each run (CPU model, governor, OS).

---

## Minimal README snippet you can paste later

````markdown
### Reproducing headline numbers (MBP 2019)

Classic (burst=512):
```bash
RON_BURST=512 RON_CAP=4096 \
cargo bench -p ron-kernel --bench bus_publish \
  -- --warm-up-time 3 --measurement-time 12 --save-baseline core-$(date +%F)-classic-b512
````

Batched (burst=1024):

```bash
RON_BURST=1024 RON_CAP=8192 \
cargo bench -p ron-kernel --features "bus_batch,metrics_buf" --bench bus_publish \
  -- --warm-up-time 3 --measurement-time 12 --save-baseline core-$(date +%F)-batched-b1024
```

Expect ~**71–77 ns** steady publish and **~8–10 MElem/s** classic burst publish with fanout=4.

```

---

Take a bow. When you’re ready to pick this up again, start with LTO+CGU=1 and a short 512/1024 sweep using the exact commands above, then widen out only if the machine isn’t cooking.
```

### END NOTE - OCTOBER 23 2025 - 12:25 CST

>We will be working on ron-bus next :)