

---

# ron-kernel — scaffold (v2)

```
crates/ron-kernel/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ deny.toml
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ CHANGELOG.md
├─ README.md
├─ API.md
├─ .cargo/
│  └─ config.toml
├─ .github/
│  └─ workflows/
│     ├─ kernel-ci.yml
│     └─ rust.yml
├─ scripts/
│  ├─ ci_public_api.sh
│  └─ render_mermaid.sh
├─ docs/
│  ├─ arch.mmd
│  ├─ seq.mmd
│  ├─ state.mmd
│  ├─ CONCURRENCY.md
│  ├─ PERFORMANCE.md
│  ├─ SECURITY.md
│  ├─ QUANTUM.md
│  ├─ OBSERVABILITY.md
│  ├─ IDB.md
│  ├─ INTEROP.md
│  └─ specs/
│     └─ OAP-1.md
├─ specs/
│  ├─ supervisor.tla
│  └─ bus.tla
├─ fuzz/
│  └─ cfg_parser.rs
├─ testing/
│  └─ performance/
│     ├─ publish_matrix.toml
│     └─ README.md
├─ examples/
│  └─ minimal_supervision.rs
├─ benches/
│  └─ bus_lag_vs_publish.rs
├─ tests/
│  ├─ public_api.rs
│  ├─ bus_bounded.rs
│  ├─ readiness_degrades.rs
│  ├─ amnesia_label.rs
│  ├─ supervisor_backoff.rs
│  ├─ tls_type_invariance.rs
│  ├─ loom_bus.rs
│  └─ property_config.rs
└─ src/
   ├─ lib.rs
   ├─ bus/
   │  ├─ mod.rs
   │  ├─ bounded.rs
   │  └─ topic.rs
   ├─ events.rs
   ├─ metrics/
   │  ├─ mod.rs
   │  ├─ exporter.rs
   │  ├─ health.rs
   │  └─ readiness.rs
   ├─ config/
   │  ├─ mod.rs
   │  ├─ watcher.rs
   │  └─ validation.rs
   ├─ supervisor/
   │  ├─ mod.rs
   │  ├─ backoff.rs
   │  ├─ child.rs
   │  └─ lifecycle.rs
   ├─ amnesia.rs
   ├─ shutdown.rs
   └─ internal/
      ├─ constants.rs
      └─ types.rs
```

---

## One-liner purpose for each file

### Root & meta

* **Cargo.toml** — Crate metadata, features, deps; keeps public surface tiny and stable.
* **rust-toolchain.toml** — Pins toolchain to MSRV (1.80.0) for reproducible builds/CI.
* **deny.toml** — `cargo-deny` policy for licenses/bans/advisories.
* **LICENSE-APACHE / LICENSE-MIT** — Dual-license boilerplate.
* **CHANGELOG.md** — SemVer changes; required for any public API diff.
* **README.md** — Kernel “constitution”: scope, diagrams, gates, run/ops.
* **API.md** — Frozen public API spec + SemVer rules + CI gate.

### Local config & CI

* **.cargo/config.toml** — Local rustflags/profiles; optionally deny missing docs in CI.
* **.github/workflows/kernel-ci.yml** — CI for `cargo public-api` deny, Mermaid render, semver checks.
* **.github/workflows/rust.yml** — Build/test/clippy/deny; adds amnesia on/off and loom lanes.

### Scripts

* **scripts/ci_public_api.sh** — Generates/compares public API snapshot (developer helper).
* **scripts/render_mermaid.sh** — Renders `docs/*.mmd` to SVG locally.

### Docs (Six Concerns spine + diagrams)

* **docs/arch.mmd** — Architecture Mermaid (kernel, bus, metrics).
* **docs/seq.mmd** — Sequence diagram (config update/event path).
* **docs/state.mmd** — Supervisor lifecycle state diagram.
* **docs/CONCURRENCY.md** — Kernel concurrency rules (no locks across `.await`, one receiver per task).
* **docs/PERFORMANCE.md** — SLOs, perf methodology, bench harness notes.
* **docs/SECURITY.md** — TLS type boundary, amnesia implications, no secrets in kernel.
* **docs/QUANTUM.md** — PQ-readiness: pass-through flags via Config; kernel is crypto-agnostic.
* **docs/OBSERVABILITY.md** — Golden metrics & readiness/liveness semantics.
* **docs/IDB.md** — Invariants/Principles/How/Proof with acceptance checklists.
* **docs/INTEROP.md** — Boundaries: what kernel exposes vs. forbids (no overlay/DHT/storage).
* **docs/specs/OAP-1.md** — Pointer to external OAP/1 spec (kernel does not enforce).

### Formal specs

* **specs/supervisor.tla** — TLA+ sketch for crash-only restarts/backoff properties.
* **specs/bus.tla** — TLA+ sketch for bounded broadcast/no lost invariants (under defined limits).

### Fuzz

* **fuzz/cfg_parser.rs** — Stub target for config validation/precedence fuzzing (optional lane).

### Performance harness

* **testing/performance/publish_matrix.toml** — Input matrix (publish rates, fanout, durations).
* **testing/performance/README.md** — How to run/interpret perf runs for SLO proofs.

### Examples & benches

* **examples/minimal_supervision.rs** — Tiny example showing startup + graceful shutdown.
* **benches/bus_lag_vs_publish.rs** — Criterion bench: publish rate vs. observed bus lag.

### Tests (proofs of invariants)

* **tests/public_api.rs** — Guards frozen re-export list (API drift fails CI).
* **tests/bus_bounded.rs** — Proves bounded bus + lag/drop metrics under saturation.
* **tests/readiness_degrades.rs** — Shows `/readyz` degrades before collapse during stress.
* **tests/amnesia_label.rs** — Ensures `amnesia="on|off"` appears in metrics; mode propagates.
* **tests/supervisor_backoff.rs** — Verifies jittered backoff + `ServiceCrashed{reason}` emission.
* **tests/tls_type_invariance.rs** — Ensures no `rustls::ServerConfig` leaks; only tokio-rustls type allowed downstream.
* **tests/loom_bus.rs** — Loom interleavings for bus/supervisor hot paths (ignored by default).
* **tests/property_config.rs** — Proptest for config precedence/merge (env > file > defaults).

### Source (modular, no large files)

* **src/lib.rs** — `#![forbid(unsafe_code)]`; `#![deny(missing_docs)]` (optional); re-exports the **only** public API: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c`.
* **src/bus/mod.rs** — Facade for in-process broadcast bus (bounded, cloneable).
* **src/bus/bounded.rs** — Capacity/overflow policy and internal counters for lag/drop.
* **src/bus/topic.rs** — Topic/predicate helpers for selective receive (internal).
* **src/events.rs** — Defines `KernelEvent` shape (frozen) and internal conversions.
* **src/metrics/mod.rs** — Golden metrics registry/types and wiring surface.
* **src/metrics/exporter.rs** — Prometheus exporter task (returns join handle + bind addr).
* **src/metrics/health.rs** — Liveness state (`/healthz`) plumbing.
* **src/metrics/readiness.rs** — Readiness state (`/readyz`) with saturation/degeneration logic.
* **src/config/mod.rs** — `Config` type (validated, hot-swappable snapshot).
* **src/config/watcher.rs** — Atomic hot-reload + emit `KernelEvent::ConfigUpdated`.
* **src/config/validation.rs** — Defaults/guards; pure validation (no I/O).
* **src/supervisor/mod.rs** — Supervisor entry; crash-only semantics; no locks across `.await`.
* **src/supervisor/backoff.rs** — Jittered backoff with intensity caps; restart metrics.
* **src/supervisor/child.rs** — Child task runner; on panic: emit `ServiceCrashed{reason}`.
* **src/supervisor/lifecycle.rs** — State transitions that drive readiness and shutdown.
* **src/amnesia.rs** — Global Amnesia Mode snapshot + metrics label helper.
* **src/shutdown.rs** — `wait_for_ctrl_c()` implementation (cooperative termination).
* **src/internal/constants.rs** — Internal tunables; OAP constants as comments with link (no enforcement).
* **src/internal/types.rs** — Small internal structs/enums shared across modules (private).

---

## Tiny checklist (what to do with each area)

* **Docs**: Fill stubs (CONCURRENCY/PERFORMANCE/SECURITY/QUANTUM) with 1–2 pages each; keep pointers to canonical blueprints.
* **CI**: Enable `kernel-ci.yml` lanes (public-api deny, Mermaid render, semver checks, amnesia matrix, loom lane).
* **Tests**: Land the invariant tests first (public_api, bus_bounded, readiness_degrades, amnesia_label).
* **Perf**: Add one measured run to `testing/performance/README.md` to set baseline SLOs.
* **Specs**: Commit minimal TLA+ drafts; they can evolve, but existing tests should reference their properties.

