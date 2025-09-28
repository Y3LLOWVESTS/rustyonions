
---

# ryker — crate map (finalized scaffold)

```
crates/ryker/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-MIT
├─ LICENSE-APACHE
├─ rust-toolchain.toml
├─ .clippy.toml
├─ .gitignore
├─ ryker.example.toml
├─ scripts/
│  ├─ render-mermaid.sh
│  ├─ public_api_snapshot.sh
│  └─ dev-notes.md
├─ docs/
│  ├─ API.md
│  ├─ CONFIG.md
│  ├─ CONCURRENCY.md
│  ├─ GOVERNANCE.md
│  ├─ IDB.md
│  ├─ INTEROP.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  ├─ SECURITY.md
│  ├─ TESTS.md
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  ├─ state.mmd
│  ├─ schemas/
│  │  └─ ryker.config.schema.json
│  ├─ api-history/
│  │  └─ ryker/
│  │     └─ vX.Y.Z.txt
│  ├─ governance_history.md
│  └─ benches/
│     └─ README.md
├─ src/
│  ├─ lib.rs
│  ├─ prelude.rs
│  ├─ errors.rs
│  ├─ config/
│  │  ├─ mod.rs
│  │  ├─ model.rs
│  │  ├─ loader.rs
│  │  └─ reload.rs
│  ├─ runtime/
│  │  ├─ mod.rs
│  │  └─ runtime.rs
│  ├─ supervisor/
│  │  ├─ mod.rs
│  │  ├─ backoff.rs
│  │  └─ supervisor.rs
│  ├─ mailbox/
│  │  ├─ mod.rs
│  │  ├─ builder.rs
│  │  ├─ queue.rs
│  │  ├─ observer.rs
│  │  └─ error.rs
│  └─ observe/
│     ├─ mod.rs
│     ├─ metrics.rs
│     └─ trace.rs
├─ examples/
│  ├─ actor_loop.rs
│  └─ config_dump.rs
├─ tests/
│  ├─ integration/
│  │  ├─ config_env_snapshot.rs
│  │  ├─ backpressure.rs
│  │  ├─ deadline.rs
│  │  ├─ reload_hot_cold.rs
│  │  ├─ amnesia.rs
│  │  ├─ supervisor_backoff.rs
│  │  └─ metrics_contract.rs
│  ├─ feature_matrix.rs
│  ├─ loom/
│  │  ├─ loom_mailbox_basic.rs
│  │  ├─ loom_shutdown.rs
│  │  └─ loom_backpressure.rs
│  └─ vectors/
│     ├─ env/
│     └─ snapshots/
│        ├─ config_snapshot.toml
│        ├─ config_snapshot.json
│        └─ metrics_contract.txt
├─ benches/
│  ├─ enqueue.rs
│  ├─ dequeue.rs
│  └─ batch.rs
└─ fuzz/
   └─ fuzz_targets/
      ├─ fuzz_parse_config_toml.rs
      ├─ fuzz_parse_config_json.rs
      └─ fuzz_mailbox_ops.rs
```

---

## What each file/folder is for (and why it exists)

### Roots / metadata

* **Cargo.toml** — Library-only crate; features: `metrics`, `tracing`, `amnesia` (default on), `loom`, `dev-cli` (off by default). Declares docsrs metadata and ensures no HTTP/TLS/DB deps leak into the public API.
* **README.md** — Role-aware canonical doc with architecture/sequence/state diagrams, acceptance gates, metrics contract, runnable examples, and troubleshooting.
* **CHANGELOG.md** — SemVer source of truth; breaking changes + migration notes.
* **LICENSE-MIT / LICENSE-APACHE** — Dual license (MIT / Apache-2.0).
* **rust-toolchain.toml** — Pins MSRV (1.80.0) for reproducible CI/dev.
* **.clippy.toml** — Enforces hygiene: `await_holding_lock` deny, plus API-size and enum-variant checks.
* **.gitignore** — Ignores target/, Criterion outputs, fuzz artifacts, and generated SVGs (if any).
* **ryker.example.toml** — Minimal, valid config demonstrating precedence and safe defaults for quick adoption.

### scripts/

* **render-mermaid.sh** — Render `docs/*.mmd` → SVG locally (same as CI recipe).
* **public_api_snapshot.sh** — Helper to regenerate `docs/api-history/ryker/vX.Y.Z.txt` using `cargo public-api --simplified`.
* **dev-notes.md** — Short how-tos: loom run patterns, fuzz cadence, bench baseline comparisons, common cargo invocations.

### docs/ (docs-as-constitution)

* **API.md** — Public surface contract (re-exports, stability promises).
* **CONFIG.md** — Builder → Env (`RYKER_*`) → File → Defaults precedence; hot vs cold field matrix; prod-guard notes.
* **CONCURRENCY.md** — Task/channel rules, shutdown choreography, “no lock across `.await`”, and single-consumer invariants.
* **GOVERNANCE.md** — Decision rules and appeal paths for invariant changes.
* **IDB.md** — Invariant-Driven Blueprinting: MUST/SHOULD/HOW/PROOF/Anti-scope.
* **INTEROP.md** — Firm boundary: no sockets, no TLS, no DBs; how hosts integrate metrics/trace hooks.
* **OBSERVABILITY.md** — Canonical metric names/labels, readiness semantics (host-exposed), tracing fields.
* **PERFORMANCE.md** — Bench protocols, target p50/p99, A/B span cost methodology.
* **QUANTUM.md** — PQ exposure = N/A for ryker; amnesia/zeroize scope clarified.
* **RUNBOOK.md** — Operator notes (even as a lib): common symptoms and diagnostic sequence.
* **SECURITY.md** — Threat model, amnesia semantics, supply-chain stance.
* **TESTS.md** — Bronze→Silver→Gold gates; loom/fuzz cadence; vectors policy.
* **arch.mmd / sequence.mmd / state.mmd** — Mermaid sources for the three required diagrams (rendered in CI).
* **schemas/ryker.config.schema.json** — JSON Schema for external tool validation of configs.
* **api-history/ryker/vX.Y.Z.txt** — `cargo public-api` snapshots; updated on surface changes.
* **governance_history.md** — Log of resolved disputes / invariant amendments.
* **benches/README.md** — Where baselines live, how to compare Criterion runs.

### src/ (small, focused, no god files)

* **lib.rs** — Thin facade; re-exports curated public surface; docstring mirrors README invariants. No global executors or IO.
* **prelude.rs** — Ergonomic re-exports (`RykerConfig`, `Runtime`, `Mailbox`, `Supervisor`) without leaking internal modules.
* **errors.rs** — Stable error taxonomy (`Busy`, `TooLarge`, `Closed`, `Timeout`, `ConfigError::*`), with host-facing mapping guidance.

#### src/config/

* **mod.rs** — Public entry; re-exports and crate-level examples for `from_env_validated()`.
* **model.rs** — Typed config: `RykerConfig { capacity, max_msg_bytes, deadlines, fairness, supervisor, amnesia, observe }` with strict validation (bounds, deny unknown fields).
* **loader.rs** — Builder/Env/File merge logic, deprecation aliases with warnings, prod-guard for file paths; deterministic snapshots.
* **reload.rs** — `RykerReloadHook` trait; diffs (redacted), counters (`ryker_config_reload_total`, `ryker_config_reload_errors_total`).

#### src/runtime/

* **mod.rs** — Public `Runtime` constructor; returns builders; runtime-agnostic API (does not own tokio/executors).
* **runtime.rs** — Tiny glue layer connecting config snapshots to builder construction and cooperative cancellation seams.

#### src/supervisor/

* **mod.rs** — Public `Supervisor` surface docs & re-exports.
* **backoff.rs** — Decorrelated jitter algorithm with base/cap/rapid-fail thresholds; easy to property-test.
* **supervisor.rs** — Restart wrapper for actor tasks (panic→restart accounting), never holding locks across `.await`.

#### src/mailbox/

* **mod.rs** — Public `Mailbox<T>` and `MailboxBuilder<T>` docs and re-exports.
* **builder.rs** — Capacity/size/deadline/fairness configuration with hot/cold enforcement (capacity & size cold-only).
* **queue.rs** — The **bounded**, single-consumer queue implementation; explicit `try_send`; counts `dropped_total{reason}`; sampled depth; zero-copy preferences.
* **observer.rs** — `MailboxObserver` hook trait with a default implementation that forwards to metrics facade.
* **error.rs** — Narrow mailbox-specific errors (e.g., `Busy`, `TooLarge`, `Closed`, `Deadline`).

#### src/observe/

* **mod.rs** — Feature gates and enable/disable knobs (e.g., queue depth sampling).
* **metrics.rs** — Canonical metric names and low-cardinality labels:
  `ryker_mailbox_depth`, `ryker_mailbox_dropped_total{reason}`, `ryker_busy_rejections_total`,
  `ryker_handler_latency_seconds{outcome}`, `ryker_actor_restarts_total`.
* **trace.rs** — Span helpers (`ryker.mailbox.enqueue`, `ryker.actor.handle`, `ryker.config.reload`) with `actor`, `deadline_ms`, `outcome`, `corr_id`.

### examples/

* **actor_loop.rs** — Minimal actor loop (pull/handle/quit); doubles as rustdoc example.
* **config_dump.rs** — Feature-gated (`dev-cli`) helper to print a validated `RykerConfig` snapshot; no prod runtime implied.

### tests/ (proofs, not vibes)

* **integration/config_env_snapshot.rs** — Asserts Builder→Env→File→Default precedence; fail-closed validation.
* **integration/backpressure.rs** — Full-queue ⇒ `Busy` + `dropped_total{reason="capacity"}` (gate: backpressure).
* **integration/deadline.rs** — Per-call/default deadlines ⇒ `outcome=timeout` in histograms (gate: deadlines).
* **integration/reload_hot_cold.rs** — Hot (deadline/fairness) vs cold (capacity/size) behavior (gate: reload semantics).
* **integration/amnesia.rs** — Amnesia on ⇒ zeroize scope smoke-checked; off ⇒ normal behavior (gate: security posture).
* **integration/supervisor_backoff.rs** — Decorrelated jitter bounds + rapid-fail ceiling (gate: restart policy).
* **integration/metrics_contract.rs** — **Golden test**: hooks capture emitted metric names/labels; compared to `tests/vectors/snapshots/metrics_contract.txt` to freeze the contract (gate: observability stability).
* **feature_matrix.rs** — Compile-only checks across feature combos (default, +tracing, +metrics, +amnesia, +loom off) to ensure surface stability and build health (gate: feature matrix).
* **loom/** — Interleavings:

  * `loom_mailbox_basic.rs` (N producers → 1 consumer; no deadlocks; FIFO per mailbox)
  * `loom_shutdown.rs` (drain correctness; cancel-safe)
  * `loom_backpressure.rs` (deterministic `Busy` under contention)
* **vectors/** — Fixtures and snapshots for config/env/metrics golden tests.

### benches/ (Criterion)

* **enqueue.rs** — Producer hot path micro-bench; amortized allocs close to zero.
* **dequeue.rs** — Consumer hot path micro-bench.
* **batch.rs** — Fairness tuning micro-bench; validates target p50/p99 under batch size changes.

### fuzz/ (cargo-fuzz targets)

* **fuzz_parse_config_toml.rs / fuzz_parse_config_json.rs** — Parser hardening (no panics, deny unknown fields).
* **fuzz_mailbox_ops.rs** — Randomized enqueue/recv/shutdown sequences for invariant validation.

---

## Guardrails to keep it maintainable

* **File size:** keep implementation files ≲ 250–300 LOC; `mod.rs` facades ≲ 80 LOC.
* **No IO / No runtime ownership:** any proposal that adds sockets/HTTP/TLS/DB goes to a host crate (not here).
* **Metrics cardinality:** new labels must be approved; `metrics_contract.rs` golden test will catch accidental drift.
* **Features:** default = `metrics`, `tracing`, `amnesia`; `loom`, `dev-cli` off by default to keep end-users lean.
* **Proof-first:** any new behavior ships with a corresponding test (integration/loom/fuzz/bench) and a doc update.

---
