
---

# `crates/ron-bus/` — final scaffold (no code)

```
crates/ron-bus/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ CODEOWNERS
├─ rust-toolchain.toml
├─ deny.toml
├─ .clippy.toml
├─ .gitignore
├─ .cargo/
│  └─ config.toml
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ coverage.yml
│     ├─ nightly-chaos.yml
│     └─ render-mermaid.yml
├─ docs/
│  ├─ README.md
│  ├─ API.md
│  ├─ CONCURRENCY.md
│  ├─ CONFIG.md
│  ├─ OBSERVABILITY.md
│  ├─ IDB.md
│  ├─ INTEROP.md
│  ├─ diagrams/
│  │  ├─ arch.mmd
│  │  ├─ sequence.mmd
│  │  └─ state.mmd
│  └─ api-history/
│     └─ ron-bus/
│        └─ v1.0.0.txt
├─ scripts/
│  └─ update_api_snapshot.sh
├─ src/
│  ├─ lib.rs
│  ├─ bus.rs
│  ├─ event.rs
│  ├─ config.rs
│  ├─ metrics.rs
│  ├─ errors.rs
│  ├─ prelude.rs
│  └─ internal/
│     ├─ channel.rs
│     ├─ depth_estimator.rs
│     └─ seals.rs
├─ tests/
│  ├─ fanout_ok.rs
│  ├─ lagged_overflow_smoke.rs
│  ├─ receiver_ownership.rs
│  ├─ capacity_cutover.rs
│  ├─ api_surface.rs
│  ├─ property_bus.rs
│  ├─ pq_labels_feature.rs
│  ├─ chaos_amnesia.rs
│  └─ loom_model.rs
└─ benches/
   ├─ throughput.rs
   ├─ latency.rs
   └─ overflow.rs
```

---

## What each file/folder is for (and why)

### Top-level governance & hygiene

* **Cargo.toml** — Declares MSRV (1.80.0), small features (`tracing`, `pq-labels`, `loom` dev-only), and pins deps to workspace. No optional I/O/logging deps to keep the core pure.
* **README.md** — Canonical source: purpose, invariants (bounded/lossy/observable; one-rx-per-task), diagrams, SLOs, acceptance gates (tests/metrics/CI). Mirrors the docs you finalized.
* **CHANGELOG.md** — SemVer changelog with migration notes; pairs with `docs/api-history` and `api_surface.rs`.
* **LICENSE-APACHE / LICENSE-MIT** — Dual licensing.
* **CODEOWNERS** — Ownership clarity (e.g., Stevan White) for reviews and audits.
* **rust-toolchain.toml** — Pins Rust **1.80.0** for reproducible builds.
* **deny.toml** — `cargo deny` policy (licenses/bans/advisories/sources) to keep supply chain clean.
* **.clippy.toml** — Strict linting (e.g., `await_holding_lock`) to enforce concurrency invariants.
* **.gitignore** — Standard ignores (`target/`, `*.svg` artifacts, coverage, generated snapshots).

### Developer Experience

* **.cargo/config.toml** — Handy aliases:

  * `lint = "clippy -p ron-bus -- -D warnings"`
  * `test-all = "test -p ron-bus --all-features"`
  * `ci-check = "fmt --all && clippy -D warnings && test -p ron-bus && deny check"`
  * `bench-all = "bench -p ron-bus"`

### CI (split for clarity & speed)

* **.github/workflows/ci.yml** — PR gate (Linux): fmt, clippy (deny warnings), unit/integration tests (incl. doc tests), `cargo public-api` snapshot check, and a quick smoke bench (not perf-gating).
* **.github/workflows/coverage.yml** — Coverage ladder (Linux only):

  * **Bronze (PRs):** ≥ 70% (hard fail)
  * **Silver (main):** ≥ 85% (hard fail)
  * **Gold (nightly):** ≥ 90% (hard fail on nightly; report-only on PRs)
* **.github/workflows/nightly-chaos.yml** — Nightly: runs `ignored` chaos tests (subscriber churn @100Hz, two-bus cutover) and Criterion compare (fail if >10% regression).
* **.github/workflows/render-mermaid.yml** — Renders `docs/diagrams/*.mmd` → `.svg` and publishes artifacts so docs/README can embed fresh diagrams.

### Documentation (short, enforceable, with sources)

* **docs/README.md** — Index/overview linking the focused set of crate docs.
* **docs/API.md** — Frozen monomorphic API surface (`Bus::{new,sender,subscribe,capacity}`, `Event::{…}`), plus guidance on additive evolution and snapshotting.
* **docs/CONCURRENCY.md** — Rules: bounded broadcast; **one receiver per task**; **no locks across `.await`**; no background tasks or I/O in the lib.
* **docs/CONFIG.md** — Capacity fixed at creation; cutover by constructing a new bus; amnesia label guidance; warn throttling.
* **docs/OBSERVABILITY.md** — Golden metrics the **host** owns/updates (`bus_overflow_dropped_total`, `bus_queue_depth`); example alert & dashboard notes.
* **docs/IDB.md** — Invariants, non-goals, proofs, and acceptance gates tied to tests and CI.
* **docs/INTEROP.md** — States the bus is **in-proc only**; all RPC/HTTP belongs to hosts; no secret/PII on the bus.
* **docs/diagrams/** — Mermaid sources for **arch**, **sequence**, **state** (rendered in CI).
* **docs/api-history/ron-bus/v1.0.0.txt** — `cargo public-api` snapshot to gate SemVer.

### Scripts (tiny, optional)

* **scripts/update_api_snapshot.sh** — One-liner helper to regenerate and store the API snapshot under `docs/api-history/ron-bus/`. Keeps SemVer checks smooth for contributors.

### Source (small, pure, monomorphic)

* **src/lib.rs** — Tiny re-export surface (`pub use bus::Bus; pub use event::Event;`), doc tests pointing to README examples. No logic here.
* **src/bus.rs** — Core: constructs **bounded** `broadcast(capacity)`, exposes `sender()` / `subscribe()` / `capacity()`. No I/O, no tasks, no logging. Enforces “capacity is fixed at creation.”
* **src/event.rs** — Canonical `Event` enum (`Health`, `ConfigUpdated`, `ServiceCrashed`, `Shutdown`), marked `#[non_exhaustive]` for safe additive growth.
* **src/config.rs** — Lightweight `BusConfig` (capacity, overflow warn, metrics ns, amnesia label) used only at construction; validates bounds; avoids runtime mutability.
* **src/metrics.rs** — **Host-owned metrics facade**: this crate only *updates* pre-registered counters/gauges; no global registry inside the lib. Optional label path for `pq-labels`.
* **src/errors.rs** — Local error taxonomy & Tokio broadcast pass-through semantics: `RecvError::Lagged(n)`, `RecvError::Closed`, `SendError` guidance.
* **src/prelude.rs** — Minimal convenience re-exports; keep tiny to avoid API sprawl.
* **src/internal/** — Private helpers to keep public files small:

  * **channel.rs** — Narrow wrapper around `tokio::sync::broadcast` with comments about invariants; never hold locks across `.await`.
  * **depth_estimator.rs** — Cheap heuristic for queue-depth gauge updates (subscriber-side).
  * **seals.rs** — Sealed traits (if any) to prevent external impls in the future; **no zeroize here** (bus stays secret-free and crypto-agnostic).

### Tests (proofs; one invariant per file)

* **tests/fanout_ok.rs** — Baseline: N subscribers, M publishes, no lag; publishers non-blocking; every subscriber receives all events.
* **tests/lagged_overflow_smoke.rs** — Force lag with tiny capacity; assert `RecvError::Lagged(n)` occurs; **counter increments**; depth gauge updates.
* **tests/receiver_ownership.rs** — Negative/behavioral: sharing a `Receiver` across tasks is disallowed by pattern; demonstrates the “one receiver per task” invariant.
* **tests/capacity_cutover.rs** — Two-bus migration under load; verify no consumer-visible gaps, depth returns to baseline, and overflow doesn’t spike on cutover.
* **tests/api_surface.rs** — Checks public API against `docs/api-history` snapshot; fails on breaking changes without SemVer bump.
* **tests/property_bus.rs** — Property tests (small data) for publish/receive ordering under no-lag and bounded overwrite behavior under backpressure.
* **tests/pq_labels_feature.rs** — Run **with** `--features pq-labels`; assert metrics labels present and **no semantic/perf delta** (±5%) vs baseline.
* **tests/chaos_amnesia.rs** *(ignored; nightly)* — With `amnesia="on"`: long-run lag churn + cutover; assert **no payload content** is logged/persisted; allocs/op envelope unchanged (±5%); overflow bounded under target.
* **tests/loom_model.rs** *(dev-only; `--features loom`)* — Minimal interleaving model (2 pubs, 2 subs, capacity=2) to detect deadlocks/order violations and lock-across-await hazards.

### Benches (split for clarity; reproducible)

* **benches/throughput.rs** — Events/sec at steady-state; target ≥ 1M ev/s on an 8-core commodity host.
* **benches/latency.rs** — p50/p95/p99 publish→recv; target p95 < 50µs; documents machine profile and flags.
* **benches/overflow.rs** — Controlled lag scenarios; assert drop rate stays < 0.01% at target load; produces a simple perf envelope.

---

## What changed vs. the previous scaffold (your requested fixes)

* **Amnesia & PQ proofs added**: `tests/chaos_amnesia.rs` and `tests/pq_labels_feature.rs` ensure labels-only PQ and amnesia neutrality (no secret handling in core).
* **Coverage ladder**: dedicated `coverage.yml` with Bronze/Silver/Gold thresholds to avoid PR flakiness yet enforce quality.
* **Nightly chaos + Criterion compare**: `nightly-chaos.yml` runs ignored chaos tests and fails the build on >10% perf regression.
* **Bench split**: `throughput.rs` + `latency.rs` + `overflow.rs` for sharper signal and easier regression triage.
* **Matrix polish**: keep PRs on Linux for speed; run macOS/Windows smoke in nightly if you want broader scheduler coverage later (easy to add).
* **DX aliases**: `.cargo/config.toml` to shorten dev commands.
* **API snapshot workflow**: small `scripts/update_api_snapshot.sh` to keep SemVer checks straightforward for contributors.

---

## Why this is the right shape (and still tiny)

* **Scope ruthlessness:** no I/O, no background tasks, no global registries, no crypto—just the bounded bus abstraction.
* **Proof over prose:** every invariant has a named test; CI gates are explicit; perf targets have repro steps.
* **Small files:** public modules stay short; private details live under `internal/`; tests are one invariant per file.
* **Auditor-ready:** diagrams rendered in CI, API surface snapshotted, coverage thresholds laddered, supply chain pinned.

